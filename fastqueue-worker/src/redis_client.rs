use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::io::{Error, ErrorKind};

use crate::config::redis_keys;

pub struct RedisClient {
    pub(crate) conn_manager: ConnectionManager,
}

impl RedisClient {
    pub async fn new(
        redis_url: &str,
        config: Option<ConnectionManagerConfig>,
    ) -> Result<Self, Error> {
        let redis_client = get_redis_client(redis_url)?;

        let conn_manager = match config {
            Some(config) => ConnectionManager::new_with_config(redis_client, config).await,
            None => ConnectionManager::new(redis_client).await,
        }
        .map_err(|e| {
            Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create Redis connection manager: {}", e),
            )
        })?;

        Ok(Self { conn_manager })
    }

    pub async fn push_task(&self, task_blob: Vec<u8>) -> Result<(), Error> {
        let mut conn = self.conn_manager.clone();
        let _: () = redis::cmd("LPUSH")
            .arg(redis_keys::TASK_QUEUE)
            .arg(task_blob)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to push task to redis: {}", e),
                )
            })?;

        Ok(())
    }

    pub async fn mark_task_as_processing(
        &self,
        conn: &mut ConnectionManager,
    ) -> Result<Option<Vec<u8>>, Error> {
        let raw_data: Option<Vec<u8>> = redis::cmd("BLMOVE")
            .arg(redis_keys::TASK_QUEUE)
            .arg(redis_keys::PROCESSING)
            .arg("RIGHT")
            .arg("LEFT")
            .arg(1)
            .query_async(conn)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to mark the task as procesing: {}", e),
                )
            })?;
        Ok(raw_data)
    }
}

pub fn get_redis_client(redis_url: &str) -> Result<redis::Client, Error> {
    let client = redis::Client::open(redis_url).map_err(|e| {
        Error::new(
            ErrorKind::ConnectionRefused,
            format!("Failed to connect to Redis: {}", e),
        )
    })?;
    Ok(client)
}
