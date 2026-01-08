use pyo3::Python;
use pyo3::types::{PyAnyMethods, PyModule};
use std::ffi::CString;
use std::fs;
use std::io::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::info;

use redis::aio::ConnectionManagerConfig;

use crate::redis_client::RedisClient;

pub async fn run_worker(
    num_workers: usize,
    redis_url: String,
    tasks_module_path: String,
) -> Result<(), Error> {
    let redis_config =
        ConnectionManagerConfig::default().set_response_timeout(Some(Duration::from_secs(5)));
    let redis_client = Arc::new(RedisClient::new(&redis_url, Some(redis_config)).await?);

    info!("Workers: {}", num_workers);
    info!("Redis: {}", redis_url);
    info!("Tasks module path: {}", tasks_module_path);

    info!("Finding tasks to register...");
    let task_functions = get_task_functions(tasks_module_path)?;
    info!("Registering tasks: {:?}", task_functions);

    info!(
        "{}",
        "-----------------------------------------------------------------------------------------------------"
    );

    let mut workers = JoinSet::new();

    for id in 0..num_workers {
        let redis_client = Arc::clone(&redis_client);
        let mut redis_manager = redis_client.conn_manager.clone();

        workers.spawn(async move {
            loop {
                match redis_client
                    .mark_task_as_processing(&mut redis_manager)
                    .await
                {
                    Ok(Some(raw_data)) => {
                        println!("Worker {} got task ({} bytes)", id, raw_data.len());
                    }
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Worker {} redis error: {}", id, e);
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
                eprintln!("Worker crashed: {}", e);
            }
        }
    }

    Ok(())
}

fn get_task_functions(module_path: String) -> Result<Vec<String>, Error> {
    let script = fs::read_to_string("fastqueue-worker/scripts/get_functions.py")?;
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
        )?;

        let funcs: Vec<String> = module
            .getattr("list_functions")?
            .call1((module_path,))?
            .extract()?;

        Ok(funcs)
    })
}
