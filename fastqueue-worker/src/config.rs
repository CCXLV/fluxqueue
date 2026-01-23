pub mod redis_keys {
    pub const WORKERS: &str = "fastqueue:workers";
    pub const TASK_QUEUE: &str = "fastqueue:queue";
    pub const PROCESSING: &str = "fastqueue:processing";
    pub const FAILED: &str = "fastqueue:failed";
    pub const DEAD: &str = "fastqueue:dead";

    pub fn get_workers_key(queue_name: &str) -> String {
        format!("{}:{}", WORKERS, queue_name)
    }

    pub fn get_queue_key(queue_name: &str) -> String {
        format!("{}:{}", TASK_QUEUE, queue_name)
    }

    pub fn get_processing_key(queue_name: &str, worker_id: &str) -> String {
        format!("{}:{}:{}", PROCESSING, queue_name, worker_id)
    }

    pub fn get_failed_key(queue_name: &str) -> String {
        format!("{}:{}", FAILED, queue_name)
    }

    pub fn get_dead_key(queue_name: &str) -> String {
        format!("{}:{}", DEAD, queue_name)
    }
}
