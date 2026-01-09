use redis::RedisError;
use std::io::{Error, ErrorKind};

pub fn get_redis_client(redis_url: &str) -> Result<redis::Client, Error> {
    let client = redis::Client::open(redis_url).map_err(|e| {
        Error::new(
            ErrorKind::ConnectionRefused,
            format!("Failed to connect to Redis: {}", e),
        )
    })?;
    Ok(client)
}

pub async fn mark_task_as_processing(
    connection: &mut redis::aio::ConnectionManager,
) -> Result<Option<Vec<u8>>, RedisError> {
    let raw_data: Option<Vec<u8>> = redis::cmd("BLMOVE")
        .arg("fastqueue:tasks")
        .arg("fastqueue:processing")
        .arg("RIGHT")
        .arg("LEFT")
        .arg(1)
        .query_async(connection)
        .await?;
    Ok(raw_data)
}
