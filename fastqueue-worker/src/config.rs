pub mod redis_keys {
    pub const TASK_REGISTRY: &str = "fastqueue:task:";
    pub const TASK_QUEUE: &str = "fastqueue:tasks:";
    pub const PROCESSING: &str = "fastqueue:processing:";
    pub const FAILED: &str = "fastqueue:failed";
}
