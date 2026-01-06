use fastqueue_common::{Task, serialize_task_data};
use std::io::{Error, ErrorKind};

pub fn push_task(redis_client: &redis::Client, task: Task) -> Result<(), Error> {
    let mut conn = redis_client.get_connection().map_err(|e| {
        Error::new(
            ErrorKind::ConnectionRefused,
            format!("Failed to get redis connection: {}", e),
        )
    })?;

    let task_blob = serialize_task_data(&task)?;

    let _: () = redis::cmd("LPUSH")
        .arg("fastqueue:tasks")
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
