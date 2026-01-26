use anyhow::{Context, Result};
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyListMethods, PyModule, PyTuple};
use pyo3::{Bound, Py, PyAny, Python};
use pythonize::pythonize;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use rmp_serde::from_slice;
use std::ffi::CString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::Task;
use crate::logger::Logger;
use crate::redis_client::{REDIS_CONN_TIMEOUT, RedisClient};
use crate::serialize::deserialize_raw_task_data;
use crate::task::TaskRegistry;

pub async fn run_worker(
    mut shutdown: watch::Receiver<bool>,
    num_workers: usize,
    redis_url: &str,
    tasks_module_path: String,
    queue_name: &str,
) -> Result<()> {
    let redis_config =
        ConnectionManagerConfig::default().set_response_timeout(Some(REDIS_CONN_TIMEOUT));
    let redis_client = Arc::new(RedisClient::new(&redis_url, redis_config).await?);

    let queue_name_is_used = redis_client.check_queue(queue_name).await?;
    if queue_name_is_used {
        if ask_yes_or_no(&format!(
            "Queue name '{}' is already used by another worker, would you like to start this worker anyway? NOTE: This will clear previous workers data.",
            queue_name
        )) {
            // TODO: Handling it this way seems wrong, I think saving PID in redis is better, and then check whether its still running or not.
            // Without this figuring out whether there's an actual worker running with this queue name or not is hard if not impossible.
            std::process::exit(1);
        } else {
            error!(
                "Queue name '{}' is already used by another worker, exiting...",
                queue_name
            );
            std::process::exit(1);
        }
    }

    info!("Queue: {}", queue_name);
    info!("Workers: {}", num_workers);
    info!("Redis: {}", redis_url);
    info!("Tasks module path: {}", tasks_module_path);

    let task_functions = get_task_functions(tasks_module_path, queue_name)?;
    let task_names: Vec<&String> = task_functions.iter().map(|(name, _obj)| name).collect();

    info!("Tasks found: {:?}", task_names);
    info!("{}", "-".repeat(65));

    let task_registry = Arc::new(TaskRegistry::new());
    for (name, task_obj) in task_functions {
        task_registry.insert(name, task_obj)?;
    }

    let queue_name = Arc::new(queue_name.to_string());
    let mut workers = JoinSet::new();

    for local_id in 0..num_workers {
        let redis_client = Arc::clone(&redis_client);
        let redis_manager = redis_client.conn_manager.clone();

        let queue_name = Arc::clone(&queue_name);
        let shutdown = shutdown.clone();
        let task_registry = Arc::clone(&task_registry);

        redis_client
            .register_worker(&queue_name, local_id.to_string())
            .await?;

        workers.spawn(worker_loop(
            shutdown,
            queue_name,
            local_id.to_string(),
            redis_client,
            redis_manager,
            task_registry,
        ));
    }

    let janitor_queue_name = Arc::clone(&queue_name);
    let janitor_redis = Arc::clone(&redis_client);
    let janitor_manager = janitor_redis.conn_manager.clone();
    let janitor_shutdown = shutdown.clone();

    workers.spawn(janitor_loop(
        janitor_shutdown,
        janitor_queue_name,
        janitor_redis,
        janitor_manager,
    ));

    shutdown.changed().await.ok();

    while let Some(res) = workers.join_next().await {
        if let Err(e) = res {
            error!("Worker panicked: {}", e);
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    redis_client.cleanup_worker_registry(&queue_name).await?;

    Ok(())
}

async fn worker_loop(
    mut shutdown: watch::Receiver<bool>,
    queue_name: Arc<String>,
    worker_id: String,
    redis_client: Arc<RedisClient>,
    mut redis_manager: ConnectionManager,
    task_registry: Arc<TaskRegistry>,
) -> Result<()> {
    let logger = Logger::new(format!("WORKER {}", &worker_id));

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                logger.info(format_args!("Shutting down..."));
                return Ok(())
            }

            res = redis_client
                .mark_task_as_processing(&mut redis_manager, &queue_name, &worker_id)
            => {
                match res {
                    Ok(Some(raw_data)) => {
                        let task = deserialize_raw_task_data(&raw_data)?;

                        logger.info(format_args!(
                            "Received a task '{}' with a total of {} Bytes",
                            &task.name,
                            raw_data.len()
                        ));

                        let Some(task_function) = task_registry.get(&task.name) else {
                            logger.warn(format_args!("Task '{}' not found in registry. Skipping", &task.name));
                            if let Err(e) = redis_client
                                .remove_from_processing(&mut redis_manager, &queue_name, &worker_id, &raw_data)
                                .await {
                                    logger.error(format_args!("Failed to remove the task: {}", e));
                            }
                            return Ok(());
                        };

                        let task_result = run_task(&task, task_function).await;

                        match task_result {
                            Ok(_) => {
                                if let Err(e) = redis_client
                                    .remove_from_processing(&mut redis_manager, &queue_name, &worker_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to remove the task after successful run: {}", e));
                                }
                            }
                            Err(e) => {
                                logger.error(format_args!("Task '{}' failed: {}", &task.name, e));
                                if let Err(err) = redis_client
                                    .mark_as_failed(&mut redis_manager, &queue_name, &worker_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to mark the task '{}' as failed: {}", &task.name, err));
                                }
                            }
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        logger.error(format_args!("Worker {} redis error: {}", worker_id, e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

async fn janitor_loop(
    mut shutdown: watch::Receiver<bool>,
    queue_name: Arc<String>,
    redis_client: Arc<RedisClient>,
    mut redis_manager: ConnectionManager,
) -> Result<()> {
    let logger = Logger::new("JANITOR");

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                logger.info(format_args!("Shutting down..."));
                return Ok(())
            }

            res = redis_client.check_failed_tasks(&mut redis_manager, &queue_name) => {
                match res {
                    Ok(Some(raw_data)) => {
                        let task = deserialize_raw_task_data(&raw_data)?;

                        logger.info(format_args!(
                            "Received a failed task '{}': retries={}, max retries={}",
                            &task.name,
                            &task.retries,
                            &task.max_retries
                        ));

                        if task.retries == task.max_retries {
                            logger.info(format_args!(
                                "Task '{}' has reached it's max retries and will be removed from the queue",
                                &task.name
                            ));
                            return Ok(())
                        }

                        redis_client.push_task(queue_name.to_string(), raw_data).await?;
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        logger.error(format_args!("Error: {}", e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

fn get_task_functions(module_path: String, queue_name: &str) -> Result<Vec<(String, Py<PyAny>)>> {
    let script = include_str!("../scripts/get_functions.py");
    let script_cstr = CString::new(script)?;
    let filename = CString::new("get_functions.py")?;
    let module_name = CString::new("get_functions")?;

    let full_current_dir = std::env::current_dir().unwrap();
    let full_module_path = full_current_dir.join(module_path);
    let clean_module_path = normalize_path(&full_module_path);
    let project_root = full_current_dir
        .ancestors()
        .find(|p| p.join("tests").exists())
        .unwrap_or(&full_current_dir);
    let real_module_path = path_to_module_path(&project_root, &clean_module_path);

    if !clean_module_path.exists() || real_module_path.is_none() {
        error!("Tasks module path {:?} doesn't exist.", clean_module_path);
        std::process::exit(1);
    }

    let real_module_path = real_module_path.unwrap();

    Python::attach(|py| {
        let module = PyModule::from_code(
            py,
            script_cstr.as_c_str(),
            filename.as_c_str(),
            module_name.as_c_str(),
        )
        .context("Failed to import python module")?;

        let py_funcs: Bound<'_, PyDict> = module
            .getattr("list_functions")
            .context("Failed to find 'list_functions'")?
            .call1((real_module_path, queue_name))
            .context("Failed to execute 'list_functions'")?
            .cast_into::<PyDict>()
            .map_err(|_| anyhow::anyhow!("Failed to cast result to a Python Dictionary"))?;

        let funcs: Vec<_> = py_funcs
            .iter()
            .filter_map(|(key, value): (Bound<PyAny>, Bound<PyAny>)| {
                let name: String = key.extract().ok()?;
                let func: Py<PyAny> = value.unbind();
                Some((name, func))
            })
            .collect();

        Ok(funcs)
    })
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

fn path_to_module_path(current_dir: &Path, target_path: &PathBuf) -> Option<String> {
    let rel_path = target_path.strip_prefix(current_dir).ok()?;

    let mut components: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if let Some(last) = components.last_mut() {
        if let Some(pos) = last.rfind('.') {
            last.truncate(pos);
        }
    }

    Some(components.join("."))
}

async fn run_task(task: &Task, task_function: Arc<Py<PyAny>>) -> Result<()> {
    let task_args: rmpv::Value = from_slice(&task.args).context(format!(
        "Failed to deserialize task {} function args",
        task.name
    ))?;
    let task_kwargs: rmpv::Value = from_slice(&task.kwargs).context(format!(
        "Failed to deserialize task {} function kwargs",
        task.name
    ))?;

    tokio::task::spawn_blocking(move || {
        Python::attach(|py| -> Result<()> {
            let py_args = pythonize(py, &task_args).context("Failed to pythonize args")?;
            let py_kwargs = pythonize(py, &task_kwargs).context("Failed to pythonize kwargs")?;

            let args_tuple = if let Ok(list) = py_args.cast::<PyList>() {
                list.to_tuple()
            } else if let Ok(tuple) = py_args.cast::<PyTuple>() {
                tuple.clone()
            } else {
                anyhow::bail!("Args must be an array/tuple, found {}", py_args.get_type());
            };

            let kwargs_dict = py_kwargs
                .cast_into::<PyDict>()
                .map_err(|_| anyhow::anyhow!("Kwargs must be a map/dict"))?;

            let result = task_function
                .call(py, args_tuple, Some(&kwargs_dict))
                .map_err(|e| anyhow::anyhow!("Failed to call Python function: {:?}", e))?;

            let bound_result = result.bind(py);
            let is_coroutine = bound_result
                .hasattr("__await__")
                .map_err(|_| anyhow::anyhow!("Failed to check if result is awaitable"))?;

            if is_coroutine {
                let asyncio = py.import("asyncio")?;
                let run_func = asyncio.getattr("run")?;

                if !run_func.is_callable() {
                    anyhow::bail!("asyncio.run() not callable. Python 3.7+ required");
                }

                run_func.call1((result,))?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task execution panicked: {}", e))??;

    Ok(())
}

fn ask_yes_or_no(question: &str) -> bool {
    loop {
        print!("{question} [y/n]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("Please enter y or n."),
        }
    }
}
