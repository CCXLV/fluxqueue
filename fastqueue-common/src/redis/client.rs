use anyhow::{Context, Result};
use redis::Client;
use redis::aio::ConnectionManager;

use crate::get_queue_key;

pub fn get_redis_client(redis_url: &str) -> Result<Client> {
    let client = redis::Client::open(redis_url).context("Failed to connect to Redis")?;
    Ok(client)
}

pub fn push_task(conn: &mut Client, queue_name: String, task_blob: Vec<u8>) -> Result<()> {
    let queue_key = get_queue_key(&queue_name);

    let _: () = redis::cmd("LPUSH")
        .arg(queue_key)
        .arg(task_blob)
        .query(conn)
        .context("Failed to push task to redis")?;

    Ok(())
}

pub async fn push_task_async(
    conn: &mut ConnectionManager,
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
