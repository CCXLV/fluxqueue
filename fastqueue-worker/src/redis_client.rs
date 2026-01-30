use anyhow::{Context, Ok, Result};
use chrono::Utc;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::sync::Arc;
use std::time::Duration;

use crate::config::redis_keys;
use crate::serialize::deserialize_raw_task_data;
use crate::serialize_task_data;
use crate::task::Task;

pub const REDIS_QUEUE_TIMEOUT: Duration = Duration::from_secs(1);
pub const REDIS_CONN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct RedisClient {
    pub(crate) conn_manager: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str, config: ConnectionManagerConfig) -> Result<Self> {
        let redis_client = get_redis_client(redis_url)?;

        let conn_manager = ConnectionManager::new_with_config(redis_client, config)
            .await
            .context("Failed to create Redis connection manager")?;

        Ok(Self { conn_manager })
    }

    pub async fn register_worker(&self, queue_name: &str, worker_id: &str) -> Result<()> {
        let mut conn = self.conn_manager.clone();
        let workers_key = redis_keys::get_workers_key(queue_name);

        let _: () = redis::cmd("SADD")
            .arg(workers_key)
            .arg(worker_id)
            .query_async(&mut conn)
            .await
            .context("Failed to register the worker")?;

        Ok(())
    }

    pub async fn set_worker_heartbeat(
        &self,
        conn: &mut ConnectionManager,
        worker_ids: Arc<Vec<Arc<str>>>,
    ) -> Result<()> {
        for worker_id in worker_ids.iter() {
            let heartbeat_key = redis_keys::get_heartbeat_key(worker_id);

            let _: () = redis::cmd("SET")
                .arg(heartbeat_key)
                .arg(1)
                .arg("EX")
                .arg(15)
                .query_async(conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to set worker heartbeat: {}", e))?;
        }

        Ok(())
    }

    pub async fn cleanup_worker_registry(
        &self,
        queue_name: &str,
        worker_ids: Arc<Vec<Arc<str>>>,
    ) -> Result<()> {
        let mut conn = self.conn_manager.clone();
        let workers_key = redis_keys::get_workers_key(queue_name);

        for worker_id in worker_ids.iter() {
            let _: () = redis::cmd("SREM")
                .arg(&workers_key)
                .arg(worker_id.to_string())
                .query_async(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to cleanup workers registry: {}", e))?;
        }

        Ok(())
    }

    pub async fn push_task(&self, queue_name: String, task_blob: Vec<u8>) -> Result<()> {
        let mut conn = self.conn_manager.clone();
        let queue_key = redis_keys::get_queue_key(&queue_name);

        let _: () = redis::cmd("LPUSH")
            .arg(queue_key)
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
        let queue_key = redis_keys::get_queue_key(queue_name);
        let processing_key = redis_keys::get_processing_key(queue_name, worker_id);

        let raw_data: Option<Vec<u8>> = redis::cmd("BLMOVE")
            .arg(queue_key)
            .arg(processing_key)
            .arg("RIGHT")
            .arg("LEFT")
            .arg(REDIS_QUEUE_TIMEOUT.as_secs())
            .query_async(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to mark task as processing: {}", e))?;

        Ok(raw_data)
    }

    pub async fn remove_from_processing(
        &self,
        conn: &mut ConnectionManager,
        queue_name: &str,
        worker_id: &str,
        task_bytes: &[u8],
    ) -> Result<()> {
        let processing_key = redis_keys::get_processing_key(queue_name, worker_id);

        let _: () = redis::cmd("LREM")
            .arg(processing_key)
            .arg(1)
            .arg(task_bytes)
            .query_async(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove task from processing queue: {}", e))?;

        Ok(())
    }

    pub async fn mark_as_failed(
        &self,
        conn: &mut ConnectionManager,
        queue_name: &str,
        worker_id: &str,
        task_bytes: &Vec<u8>,
    ) -> Result<()> {
        let processing_key = redis_keys::get_processing_key(queue_name, worker_id);
        let failed_key = redis_keys::get_failed_key(queue_name);

        let task =
            deserialize_raw_task_data(&task_bytes).context("Failed to deserialize task data")?;

        let now = Utc::now().timestamp() as u64;
        let backoff_seconds = (30 * 2u64.pow(task.retries as u32)).min(3600);
        let retry_at = now + backoff_seconds;

        let mut cloned_task: Task = task.into();
        cloned_task.retries += 1;

        let new_task_bytes =
            serialize_task_data(&cloned_task).context("Failed to serialize task")?;

        let _: () = redis::pipe()
            .atomic()
            .zadd(failed_key, &new_task_bytes, retry_at)
            .lrem(processing_key, 1, task_bytes)
            .query_async(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to mark task as failed: {}", e))?;

        Ok(())
    }

    pub async fn check_failed_tasks(
        &self,
        conn: &mut ConnectionManager,
        queue_name: &str,
    ) -> Result<Option<Vec<u8>>> {
        let _queue_key = redis_keys::get_queue_key(queue_name);
        let failed_key = redis_keys::get_failed_key(queue_name);

        let lua_script = include_str!("../scripts/lua/pop_ready_failed.lua");
        let script = redis::Script::new(lua_script);

        let now = Utc::now().timestamp();

        let raw_data: Option<Vec<u8>> = script.key(&failed_key).arg(now).invoke_async(conn).await?;

        Ok(raw_data)
    }
}

pub fn get_redis_client(redis_url: &str) -> Result<redis::Client> {
    let client = redis::Client::open(redis_url).context("Failed to connect to Redis")?;
    Ok(client)
}
