pub mod redis_keys {
    pub const WORKERS: &str = "fastqueue:workers";
    pub const TASK_QUEUE: &str = "fastqueue:queue";
    pub const PROCESSING: &str = "fastqueue:processing";
    pub const FAILED: &str = "fastqueue:failed";
    pub const DEAD: &str = "fastqueue:dead";
}
