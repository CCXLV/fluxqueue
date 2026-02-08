use anyhow::{Context, Result};
use deadpool_redis::{Connection, redis};

use crate::get_queue_key;

pub fn get_redis_connection(redis_url: &str) -> Result<redis::Connection> {
    let client = redis::Client::open(redis_url).context("Failed to create Redis client")?;
    let conn = client
        .get_connection()
        .context("Failed to connect to Redis")?;
    Ok(conn)
}

pub fn push_task(
    conn: &mut redis::Connection,
    queue_name: String,
    task_blob: Vec<u8>,
) -> Result<()> {
    let queue_key = get_queue_key(&queue_name);

    let _: () = redis::cmd("LPUSH")
        .arg(queue_key)
        .arg(task_blob)
        .query(conn)
        .context("Failed to push task to redis")?;

    Ok(())
}

pub async fn push_task_async(
    conn: &mut Connection,
    queue_name: String,
    task_blob: Vec<u8>,
) -> Result<()> {
    let queue_key = get_queue_key(&queue_name);

    let _: () = redis::cmd("LPUSH")
        .arg(queue_key)
        .arg(task_blob)
        .query_async(conn)
        .await
        .context("Failed to push task to redis")?;

    Ok(())
}
