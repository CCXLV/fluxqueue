use anyhow::{Context, Result, anyhow};
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyListMethods, PyModule, PyTuple};
use pyo3::{Bound, Py, PyAny, Python};
use pythonize::pythonize;
use rmp_serde::from_slice;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::logger::{Logger, initial_logs};
use crate::redis_client::RedisClient;
use crate::task::{PythonDispatcher, TaskRegistry};
use fluxqueue_common::{Task, deserialize_raw_task_data};

pub async fn run_worker(
    mut shutdown: watch::Receiver<bool>,
    concurrency: usize,
    redis_url: String,
    tasks_module_path: String,
    queue_name: String,
    save_dead_tasks: bool,
) -> Result<()> {
    let redis_client = RedisClient::new(&redis_url).await.map_err(|e| {
        tracing::error!("{}", e);
        std::process::exit(1);
    })?;
    let redis_client = Arc::new(redis_client);

    let task_functions = get_task_functions(&tasks_module_path, &queue_name).map_err(|e| {
        tracing::error!("{}", e);
        std::process::exit(1);
    })?;
    let task_names: Vec<&String> = task_functions.iter().map(|(name, _obj)| name).collect();

    initial_logs(
        &queue_name,
        concurrency,
        &redis_url,
        &tasks_module_path,
        &task_names,
    );

    let task_registry = Arc::new(TaskRegistry::new());
    for (name, task_obj) in task_functions {
        task_registry.insert(name, task_obj)?;
    }

    let queue_name = Arc::from(queue_name.to_string());
    let executor_ids = generate_executor_ids(concurrency);
    let mut executors = JoinSet::new();

    for i in 0..concurrency {
        let redis_client = Arc::clone(&redis_client);
        let queue_name = Arc::clone(&queue_name);
        let executor_id = Arc::clone(&executor_ids[i]);
        let shutdown = shutdown.clone();
        let task_registry = Arc::clone(&task_registry);
        // TODO: Add a way to let the user choose between having 1 event loop per tokio task or just a single event loop in a worker.
        let python_dispatcher = Arc::new(PythonDispatcher::new());

        redis_client
            .register_executor(&queue_name, &executor_id)
            .await?;

        redis_client.set_executor_heartbeat(&executor_id).await?;

        executors.spawn(executor_loop(
            shutdown,
            queue_name,
            executor_id,
            redis_client,
            task_registry,
            python_dispatcher,
        ));
    }

    let janitor_queue_name = Arc::clone(&queue_name);
    let janitor_redis = Arc::clone(&redis_client);
    let janitor_executor_ids = Arc::clone(&executor_ids);
    let janitor_shutdown = shutdown.clone();
    let save_dead_tasks = Arc::new(save_dead_tasks);

    executors.spawn(janitor_loop(
        janitor_shutdown,
        janitor_queue_name,
        janitor_executor_ids,
        save_dead_tasks,
        janitor_redis,
    ));

    shutdown.changed().await.ok();

    while let Some(res) = executors.join_next().await {
        if let Err(e) = res {
            tracing::error!("Worker panicked: {}", e);
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    redis_client
        .cleanup_executors_registry(&queue_name, executor_ids)
        .await?;

    Ok(())
}

async fn executor_loop(
    mut shutdown: watch::Receiver<bool>,
    queue_name: Arc<str>,
    executor_id: Arc<str>,
    redis_client: Arc<RedisClient>,
    task_registry: Arc<TaskRegistry>,
    python_dispatcher: Arc<PythonDispatcher>,
) -> Result<()> {
    let logger = Logger::new(format!("EXECUTOR {}", &executor_id));

    logger.info(format_args!("Started!"));

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                logger.info(format_args!("Shutting down..."));
                return Ok(())
            }

            res = redis_client
                .mark_task_as_processing(&queue_name, &executor_id)
            => {
                match res {
                    Ok(Some(raw_data)) => {
                        let task = deserialize_raw_task_data(&raw_data)?;
                        let task_name = format!("{}:{}", &task.name, &task.id);

                        logger.info(format_args!(
                            "Received a task '{}' with a total of {} Bytes",
                            &task_name,
                            raw_data.len()
                        ));

                        let Some(task_function) = task_registry.get(&task.name) else {
                            logger.warn(format_args!("Task '{}' not found in registry. Skipping", &task.name));
                            if let Err(e) = redis_client
                                .remove_from_processing(&queue_name, &executor_id, &raw_data)
                                .await {
                                    logger.error(format_args!("Failed to remove the task: {}", e));
                            }
                            return Ok(());
                        };

                        let duration_start = Instant::now();
                        let task_result = run_task(python_dispatcher.clone(), &task, task_function).await;

                        match task_result {
                            Ok(_) => {
                                if let Err(e) = redis_client
                                    .remove_from_processing(&queue_name, &executor_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to remove the task after successful run: {}", e));
                                }
                                let duration_end = duration_start.elapsed();
                                logger.info(format_args!(
                                    "Task '{}' successfully finished in {}ms",
                                    &task_name,
                                    duration_end.as_millis()
                                ));
                            }
                            Err(e) => {
                                let duration_end = duration_start.elapsed();
                                logger.error(format_args!(
                                    "Task '{}' failed in {}ms: {}",
                                    &task_name,
                                    duration_end.as_millis(),
                                    e
                                ));
                                if let Err(err) = redis_client
                                    .mark_as_failed(&queue_name, &executor_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to mark the task '{}' as failed: {}", &task.name, err));
                                }
                            }
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        logger.error(format_args!("Worker {} redis error: {}", &executor_id, e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

async fn janitor_loop(
    mut shutdown: watch::Receiver<bool>,
    queue_name: Arc<str>,
    executor_ids: Arc<Vec<Arc<str>>>,
    save_dead_tasks: Arc<bool>,
    redis_client: Arc<RedisClient>,
) -> Result<()> {
    let logger = Logger::new("JANITOR");

    logger.info(format_args!("Started!"));

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                logger.info(format_args!("Shutting down..."));
                return Ok(())
            }

            tasks_res = async {
                redis_client.check_failed_tasks(&queue_name).await
            } => {
                match tasks_res {
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

                            if *save_dead_tasks {
                                redis_client.push_dead_task(&queue_name, raw_data).await?;
                            }

                            return Ok(())
                        }

                        redis_client.push_task(queue_name.to_string(), raw_data).await?;
                    }
                    Ok(None) => {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    },
                    Err(e) => {
                        logger.error(format_args!("Error: {}", e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }

            hearbeat = async {
                let executor_ids = Arc::clone(&executor_ids);
                redis_client
                    .set_executors_heartbeat(executor_ids)
                    .await
            } => {
                match hearbeat {
                    Ok(()) => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        logger.error(format_args!("Hearbeat Error: {}", e));
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn run_task(
    python_dispatcher: Arc<PythonDispatcher>,
    task: &Task,
    task_function: Arc<Py<PyAny>>,
) -> Result<()> {
    let task_args: rmpv::Value = from_slice(&task.args).context(format!(
        "Failed to deserialize task '{}' function args",
        task.name
    ))?;
    let task_kwargs: rmpv::Value = from_slice(&task.kwargs).context(format!(
        "Failed to deserialize task '{}' function kwargs",
        task.name
    ))?;

    let maybe_coro = Python::attach(|py| -> Result<Option<Py<PyAny>>> {
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
            .map_err(|_| anyhow!("Kwargs must be a map/dict"))?;

        let result = task_function
            .call(py, args_tuple, Some(&kwargs_dict))
            .map_err(|e| anyhow!("Failed to call Python function: {:?}", e))?;

        let bound_result = result.bind(py);
        let is_coroutine = bound_result
            .hasattr("__await__")
            .map_err(|_| anyhow!("Failed to check if result is awaitable"))?;

        if is_coroutine {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    })?;

    if let Some(coro) = maybe_coro {
        python_dispatcher.execute(coro).await?;
    }

    Ok(())
}

fn get_task_functions(module_path: &str, queue_name: &str) -> Result<Vec<(String, Py<PyAny>)>> {
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
    let real_module_path = path_to_module_path(project_root, &clean_module_path);

    if !clean_module_path.exists() || real_module_path.is_none() {
        return Err(anyhow!(
            "Tasks module path {:?} doesn't exist.",
            clean_module_path
        ));
    }

    let real_module_path = real_module_path.unwrap();
    let module_dir = project_root.to_string_lossy().to_string();

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
            .map_err(|e| anyhow!("Failed to get 'list_functions' script: {}", e))?
            .call1((real_module_path, queue_name, module_dir))
            .map_err(|e| anyhow!("Failed to get tasks: {}", e))?
            .cast_into::<PyDict>()
            .map_err(|_| anyhow!("Failed to cast result to a Python Dictionary"))?;

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

fn path_to_module_path(current_dir: &Path, target_path: &Path) -> Option<String> {
    let rel_path = target_path.strip_prefix(current_dir).ok()?;

    let mut components: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if let Some(last) = components.last_mut()
        && let Some(pos) = last.rfind('.')
    {
        last.truncate(pos);
    }

    Some(components.join("."))
}

fn generate_executor_ids(num_executors: usize) -> Arc<Vec<Arc<str>>> {
    let mut ids = Vec::with_capacity(num_executors);

    for _ in 0..num_executors {
        let id: Arc<str> = Arc::from(uuid::Uuid::new_v4().to_string());
        ids.push(id);
    }

    Arc::new(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::TestWriter;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_path_to_module_path() -> Result<()> {
        let current_dir = Path::new("project");
        let tasks_path = Path::new("../project/tasks.py");
        let normalized_path = normalize_path(tasks_path);
        let module_path = path_to_module_path(current_dir, &normalized_path);
        let expected_path = Path::new("project/tasks.py");

        assert_eq!(normalized_path, expected_path);
        assert_eq!(module_path, Some("tasks".to_string()));

        Ok(())
    }

    fn get_test_module_path(filename: &str) -> String {
        let current_dir = std::env::current_dir().unwrap();
        let test_module_path = current_dir.join("tests").join(filename);
        test_module_path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_get_task_functions_valid_module() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let functions = get_task_functions(&module_path_str, "default")?;

        assert_eq!(functions.len(), 3);

        let task_names: Vec<String> = functions.iter().map(|(name, _)| name.clone()).collect();
        assert!(task_names.contains(&"task-1".to_string()));
        assert!(task_names.contains(&"task-2".to_string()));
        assert!(task_names.contains(&"async-task".to_string()));

        assert!(!task_names.contains(&"high-priority-task".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_task_functions_different_queue() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let functions = get_task_functions(&module_path_str, "high-priority")?;

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].0, "high-priority-task");

        Ok(())
    }

    #[test]
    fn test_get_task_functions_empty_module() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_empty.py");
        let functions = get_task_functions(&module_path_str, "default")?;

        assert_eq!(functions.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_task_functions_duplicate_names() {
        let module_path_str = get_test_module_path("test_tasks_duplicate.py");

        let result = get_task_functions(&module_path_str, "default");
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("duplicated") || error_msg.contains("duplicate"));
    }

    #[test]
    fn test_get_task_functions_invalid_path() {
        let result = get_task_functions("nonexistent/path/to/module.py", "default");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_task_with_sync_function() -> Result<()> {
        let task_registry = TaskRegistry::new();
        let python_dispatcher = Arc::new(PythonDispatcher::new());
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_functions = get_task_functions(&module_path_str, "default")?;

        for (name, task_obj) in task_functions {
            task_registry.insert(name, task_obj)?;
        }

        let task = task_registry.get("task-1");
        assert!(task.is_some());

        if let Some(task_func) = task {
            let task = Task {
                id: "test-id".to_string(),
                name: "name".to_string(),
                args: vec![144],   // ()
                kwargs: vec![128], // {}
                created_at: 0,
                retries: 0,
                max_retries: 3,
            };

            let result = run_task(python_dispatcher.clone(), &task, task_func).await;
            assert!(!result.is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_run_task_with_async_function() -> Result<()> {
        let task_registry = TaskRegistry::new();
        let python_dispatcher = Arc::new(PythonDispatcher::new());
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_functions = get_task_functions(&module_path_str, "default")?;

        for (name, task_obj) in task_functions {
            task_registry.insert(name, task_obj)?;
        }

        let task = task_registry.get("async-task");
        assert!(task.is_some());

        if let Some(task_func) = task {
            let task = Task {
                id: "test-id".to_string(),
                name: "name".to_string(),
                args: vec![0x92, 0x01, 0x02], // (1, 2)
                kwargs: vec![128],
                created_at: 0,
                retries: 0,
                max_retries: 3,
            };

            let result = run_task(python_dispatcher.clone(), &task, task_func).await;
            assert!(!result.is_err());
        }

        Ok(())
    }

    async fn enqueue_tasks(redis_url: &str) -> Result<()> {
        use deadpool_redis::{Config, Runtime};
        use fluxqueue_common::{
            get_redis_connection, push_task, push_task_async, serialize_task_data,
        };

        let mut redis_conn = get_redis_connection(redis_url)?;

        let sync_task = Task {
            id: "test-id".to_string(),
            name: "task-2".to_string(),
            args: vec![0x92, 0x01, 0x02], // (1, 2)
            kwargs: vec![128],
            created_at: 0,
            retries: 0,
            max_retries: 3,
        };
        let sync_task_blob = serialize_task_data(&sync_task)?;

        push_task(&mut redis_conn, "default".to_string(), sync_task_blob)?;

        let async_task = Task {
            id: "test-id-2".to_string(),
            name: "async-task".to_string(),
            args: vec![0x92, 0x01, 0x02], // (1, 2)
            kwargs: vec![128],
            created_at: 0,
            retries: 0,
            max_retries: 3,
        };
        let async_task_blob = serialize_task_data(&async_task)?;

        let redis_config = Config::from_url(redis_url);
        let redis_pool = redis_config.create_pool(Some(Runtime::Tokio1)).unwrap();
        let mut conn_manager = redis_pool.get().await?;

        push_task_async(&mut conn_manager, "default".to_string(), async_task_blob).await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_run_worker() -> Result<()> {
        use tokio::sync::watch;
        use tokio::time::{Duration, sleep};

        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = logs.clone();

        let subscriber = tracing_subscriber::fmt()
            .with_target(false)
            .with_writer(move || TestWriter {
                logs: logs_clone.clone(),
            })
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);

        let module_path_str = get_test_module_path("test_tasks_module.py");
        let redis_url = "redis://localhost:6379";

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let worker_handle = tokio::spawn(run_worker(
            shutdown_rx,
            4,
            redis_url.to_string(),
            module_path_str,
            "default".to_string(),
            false,
        ));

        sleep(Duration::from_secs(5)).await;
        enqueue_tasks(&redis_url).await?;
        sleep(Duration::from_secs(5)).await;

        let _ = shutdown_tx.send(true);

        worker_handle.await??;

        let log_lines = logs.lock().unwrap();
        let error_found = log_lines.iter().any(|line| line.contains("ERROR"));

        assert!(!error_found, "Worker emitted an error log!");

        Ok(())
    }
}
