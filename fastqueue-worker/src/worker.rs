use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyModule};
use pyo3::{Bound, Py, PyAny, Python};
use std::ffi::CString;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{error, info};
use uuid::Uuid;

use redis::aio::ConnectionManagerConfig;

use crate::redis_client::RedisClient;
use crate::task::TaskRegistry;

pub async fn run_worker(
    num_workers: usize,
    redis_url: String,
    tasks_module_path: String,
    queue_name: String,
) -> Result<(), Error> {
    let redis_config =
        ConnectionManagerConfig::default().set_response_timeout(Some(Duration::from_secs(5)));
    let redis_client = Arc::new(RedisClient::new(&redis_url, Some(redis_config)).await?);

    info!("Queue: {}", queue_name);
    info!("Workers: {}", num_workers);
    info!("Redis: {}", redis_url);
    info!("Tasks module path: {}", tasks_module_path);

    info!("Finding tasks to register...");
    let task_functions = get_task_functions(tasks_module_path)?;
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

    let queue_name = Arc::new(queue_name);
    let instance_id = Uuid::new_v4().to_string();
    let mut workers = JoinSet::new();

    for local_id in 0..num_workers {
        let redis_client = Arc::clone(&redis_client);
        let mut redis_manager = redis_client.conn_manager.clone();

        let queue_name = Arc::clone(&queue_name);
        let worker_id = format!("{}:{}", instance_id, local_id);

        workers.spawn(async move {
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

                match redis_client
                    .mark_task_as_processing(&mut redis_manager, &queue_name, &worker_id)
                    .await
                {
                    Ok(Some(raw_data)) => {
                        info!("Worker {} got task ({} bytes)", worker_id, raw_data.len());
                    }
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        error!("Worker {} redis error: {}", worker_id, e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue;
                    }
                }
            }
        });
    }

    while let Some(res) = workers.join_next().await {
        match res {
            Ok(_) => {
                // worker finished cleanly, nothing to log
            }
            Err(e) => {
                error!("Worker crashed: {}", e);
            }
        }
    }

    Ok(())
}

fn get_task_functions(module_path: String) -> Result<Vec<(String, Py<PyAny>)>, Error> {
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
            .call1((module_path,))
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
