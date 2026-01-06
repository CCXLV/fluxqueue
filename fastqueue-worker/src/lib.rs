use redis::aio::ConnectionManagerConfig;
use std::{io::Error, time::Duration};
use tokio::task::JoinSet;

mod redis_client;
mod serialize;
mod task;
mod worker;

pub use redis_client::*;
pub use serialize::*;
pub use task::*;

pub async fn start_worker(redis_url: String, num_workers: u32) -> Result<(), Error> {
    let redis_client = get_redis_client(&redis_url)?;
    let config =
        ConnectionManagerConfig::default().set_response_timeout(Some(Duration::from_secs(5)));

    let redis_manager = redis::aio::ConnectionManager::new_with_config(redis_client, config)
        .await
        .map_err(|e| {
            Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create connection manager: {}", e),
            )
        })?;
    let mut workers = JoinSet::new();

    for id in 0..num_workers {
        let mut manager = redis_manager.clone();

        workers.spawn(async move {
            loop {
                match mark_task_as_processing(&mut manager).await {
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
