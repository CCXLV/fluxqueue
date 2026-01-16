use anyhow::{Context, Result};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};

use crate::config::redis_keys;

pub struct RedisClient {
    pub(crate) conn_manager: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str, config: Option<ConnectionManagerConfig>) -> Result<Self> {
        let redis_client = get_redis_client(redis_url)?;

        let conn_manager = match config {
            Some(config) => ConnectionManager::new_with_config(redis_client, config).await,
            None => ConnectionManager::new(redis_client).await,
        }
        .context("Failed to create Redis connection manager")?;

        Ok(Self { conn_manager })
    }

    pub async fn register_worker(&self, queue_name: &str, worker_id: String) -> Result<()> {
        let mut conn = self.conn_manager.clone();
        let script = include_str!("../scripts/lua/register_worker.lua");

        let _: () = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(format!("{}:{}", redis_keys::WORKERS, queue_name))
            .arg(worker_id)
            .query_async(&mut conn)
            .await
            .context("Failed to register the worker")?;

        Ok(())
    }

    pub async fn check_queue(&self, queue_name: &str) -> Result<bool> {
        let mut conn = self.conn_manager.clone();
        let queue: bool = redis::cmd("EXISTS")
            .arg(format!("{}:{}", redis_keys::WORKERS, queue_name))
            .query_async(&mut conn)
            .await
            .context("Failed to check queue name")?;
        Ok(queue)
    }

    pub async fn cleanup_worker_registry(&self, queue_name: &str) -> Result<usize> {
        let mut conn = self.conn_manager.clone();
        let script = include_str!("../scripts/lua/cleanup_worker_registry.lua");

        let result: usize = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(format!("{}:{}", redis_keys::WORKERS, queue_name))
            .query_async(&mut conn)
            .await
            .context("Failed to cleanup the worker registry")?;

        Ok(result)
    }

    pub async fn push_task(&self, queue_name: String, task_blob: Vec<u8>) -> Result<()> {
        let mut conn = self.conn_manager.clone();
        let _: () = redis::cmd("LPUSH")
            .arg(format!("{}:{}", redis_keys::TASK_QUEUE, queue_name))
            .arg(task_blob)
            .query_async(&mut conn)
            .await
            .context("Failed to push task to redis")?;

        Ok(())
    }

    pub async fn mark_task_as_processing(
        &self,
        conn: &mut ConnectionManager,
        queue_name: &str,
        worker_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let raw_data: Option<Vec<u8>> = redis::cmd("BLMOVE")
            .arg(format!("{}:{}", redis_keys::TASK_QUEUE, queue_name))
            .arg(format!(
                "{}:{}:{}",
                redis_keys::PROCESSING,
                queue_name,
                worker_id
            ))
            .arg("RIGHT")
            .arg("LEFT")
            .arg(1)
            .query_async(conn)
            .await
            .context("Failed to mark the task as processing")?;

        Ok(raw_data)
    }

    pub async fn remove_from_processing(
        &self,
        conn: &mut ConnectionManager,
        queue_name: &str,
        worker_id: &str,
        task_bytes: &[u8],
    ) -> Result<()> {
        let processing_key = format!("{}:{}:{}", redis_keys::PROCESSING, queue_name, worker_id);

        let _: () = redis::cmd("LREM")
            .arg(&processing_key)
            .arg(1)
            .arg(task_bytes)
            .query_async(conn)
            .await
            .context("Failed to remove task from processing")?;

        Ok(())
    }
}

pub fn get_redis_client(redis_url: &str) -> Result<redis::Client> {
    let client = redis::Client::open(redis_url).context("Failed to connect to Redis")?;
    Ok(client)
}
