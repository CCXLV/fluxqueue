use fastqueue_worker::{get_redis_client, redis_keys};
use std::io::{Error, ErrorKind};

/// Synchronous RedisClient for core usage.
pub struct RedisClient {
    pub(crate) client: redis::Client,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> Result<Self, Error> {
        let client = get_redis_client(&redis_url)?;

        Ok(Self { client })
    }

    pub fn push_task(&self, queue_name: String, task_blob: Vec<u8>) -> Result<(), Error> {
        let mut conn = self.client.clone();
        let _: () = redis::cmd("LPUSH")
            .arg(format!("{}:{}", redis_keys::TASK_QUEUE, queue_name))
            .arg(task_blob)
            .query(&mut conn)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to push task to redis: {}", e),
                )
            })?;
        Ok(())
    }
}
