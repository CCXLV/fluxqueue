static EXECUTORS: &str = "fastqueue:executors";
static HEARTBEAT: &str = "fastqueue:heartbeat";
static TASK_QUEUE: &str = "fastqueue:queue";
static PROCESSING: &str = "fastqueue:processing";
static FAILED: &str = "fastqueue:failed";
static DEAD: &str = "fastqueue:dead";

pub fn get_executors_key(queue_name: &str) -> String {
    format!("{}:{}", EXECUTORS, queue_name)
}

pub fn get_heartbeat_key(executor_id: &str) -> String {
    format!("{}:{}", HEARTBEAT, executor_id)
}

pub fn get_queue_key(queue_name: &str) -> String {
    format!("{}:{}", TASK_QUEUE, queue_name)
}

pub fn get_processing_key(queue_name: &str, executor_id: &str) -> String {
    format!("{}:{}:{}", PROCESSING, queue_name, executor_id)
}

pub fn get_failed_key(queue_name: &str) -> String {
    format!("{}:{}", FAILED, queue_name)
}

pub fn get_dead_key(queue_name: &str) -> String {
    format!("{}:{}", DEAD, queue_name)
}
