pub mod redis_keys {
    pub const WORKERS: &str = "fastqueue:workers";
    pub const TASK_QUEUE: &str = "fastqueue:queue";
    pub const PROCESSING: &str = "fastqueue:processing";
    pub const FAILED: &str = "fastqueue:failed";
    pub const DEAD: &str = "fastqueue:dead";
}

pub fn get_task_key(queue_name: String) -> String {
    format!("{}:{}", redis_keys::TASK_QUEUE, queue_name)
}
