use std::io::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::info;

use redis::aio::ConnectionManagerConfig;

use crate::redis_client::RedisClient;

pub async fn run_worker(redis_url: String, num_workers: usize) -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter("fastqueue_worker=debug")
        .with_target(false)
        .init();

    info!("Starting FastQueue worker");
    info!("Workers: {}", num_workers);
    info!("Redis: {}", redis_url);
    info!("{}", "-".repeat(35));

    let redis_config =
        ConnectionManagerConfig::default().set_response_timeout(Some(Duration::from_secs(5)));
    let redis_client = Arc::new(RedisClient::new(&redis_url, Some(redis_config)).await?);

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
