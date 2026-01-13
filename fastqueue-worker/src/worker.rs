use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyModule};
use pyo3::{Bound, Py, PyAny, Python};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::ffi::CString;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tracing::{error, info};
use uuid::Uuid;

use crate::redis_client::RedisClient;
use crate::task::TaskRegistry;

pub async fn run_worker(
    mut shutdown: watch::Receiver<bool>,
    num_workers: usize,
    redis_url: &str,
    tasks_module_path: String,
    queue_name: &str,
) -> Result<(), Error> {
    let redis_config =
        ConnectionManagerConfig::default().set_response_timeout(Some(Duration::from_secs(5)));
    let redis_client = Arc::new(RedisClient::new(&redis_url, Some(redis_config)).await?);

    let queue_name_is_used = redis_client.check_queue(queue_name).await?;
    if queue_name_is_used {
        error!(
            "Queue name '{}' is already used by another worker, exiting...",
            queue_name
        );
        std::process::exit(1);
    }

    info!("Queue: {}", queue_name);
    info!("Workers: {}", num_workers);
    info!("Redis: {}", redis_url);
    info!("Tasks module path: {}", tasks_module_path);

    info!("Finding tasks to register...");
    let task_functions = get_task_functions(tasks_module_path, queue_name)?;
    let task_names: Vec<&String> = task_functions.iter().map(|(name, _obj)| name).collect();

    info!("Registering tasks: {:?}", task_names);

    info!(
        "{}",
        "-----------------------------------------------------------------------------------------------------"
    );

    let task_registry = TaskRegistry::new();
    for (name, task_obj) in task_functions {
        task_registry.insert(name, task_obj)?;
    }

    let queue_name = Arc::new(queue_name.to_string());
    // TODO: using newly generated uuid on every startup
    // wont let the worker run the tasks in the processing queue
    // if the worker has been shut down before running the functions
    let instance_id = Uuid::new_v4().to_string();
    let mut workers = JoinSet::new();

    for local_id in 0..num_workers {
        let redis_client = Arc::clone(&redis_client);
        let redis_manager = redis_client.conn_manager.clone();

        let queue_name = Arc::clone(&queue_name);
        let worker_id = format!("{}:{}", instance_id, local_id);
        let shutdown = shutdown.clone();

        redis_client
            .register_worker(&queue_name, &worker_id)
            .await?;

        workers.spawn(worker_loop(
            shutdown,
            queue_name,
            worker_id,
            redis_client,
            redis_manager,
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

    redis_client.cleanup_worker_registry(&queue_name).await?;

    Ok(())
}

async fn worker_loop(
    mut shutdown: watch::Receiver<bool>,
    queue_name: Arc<String>,
    worker_id: String,
    redis_client: Arc<RedisClient>,
    mut redis_manager: ConnectionManager,
) {
    loop {
        // The process
        // 1. Task comes in -> check if its in task_registry or not
        //      If its not in task registry just warn the user and continue the loop
        //      Task functions must be registered before proceeding with its processing
        // 2. Task is getting marked as processing and getting the function arguments
        // 3. Using the registered function, and the arguments and running the function
        //      Will add some more about this when I get there.
        // 4. If function failed to run, we mark that task as FAILED and put it in Redis Sorted Set
        // 5. Run second lightweight loop (Janitor) for managing failed tasks
        //      The loop should wake up in every 1 second and check whether there are any failed tasks or not
        //      If there are it should check its time when to retry basically it would be like this current_time + (current_retry * 4) minutes
        //      If its time to retry the task move the task back to the queue list, and just repead the process.

        tokio::select! {
            _ = shutdown.changed() => {
                info!("Worker {} shutting down...", worker_id);
                break;
            }

            res = redis_client
                .mark_task_as_processing(&mut redis_manager, &queue_name, &worker_id)
            => {
                match res {
                    Ok(Some(raw_data)) => {
                        info!("Worker {} got task ({} bytes)", worker_id, raw_data.len());
                    }
                    Ok(None) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("Worker {} redis error: {}", worker_id, e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

async fn janitor_loop(
    mut shutdown: watch::Receiver<bool>,
    _queue_name: Arc<String>,
    _redis_client: Arc<RedisClient>,
    mut _redis_manager: ConnectionManager,
) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                info!("Janitor shutting down...");
                break;
            }
        }
    }
}

fn get_task_functions(
    module_path: String,
    queue_name: &str,
) -> Result<Vec<(String, Py<PyAny>)>, Error> {
    let script = include_str!("../scripts/get_functions.py");
    let script_cstr = CString::new(script)?;
    let filename = CString::new("get_functions.py")?;
    let module_name = CString::new("get_functions")?;

    Python::initialize();
    Python::attach(|py| {
        let module = PyModule::from_code(
            py,
            script_cstr.as_c_str(),
            filename.as_c_str(),
            module_name.as_c_str(),
        )
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let py_funcs: Bound<'_, PyDict> = module
            .getattr("list_functions")
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?
            .call1((module_path, queue_name))
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?
            .cast_into()
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

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
