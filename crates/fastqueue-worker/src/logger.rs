use std::fmt::Arguments;
use tracing::{error, info, warn};

pub struct Logger {
    label: String,
}

impl Logger {
    pub fn new<S: Into<String>>(label: S) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn info(&self, args: Arguments) {
        info!("[{}]: {}", self.label, args);
    }

    pub fn warn(&self, args: Arguments) {
        warn!("[{}]: {}", self.label, args);
    }

    pub fn error(&self, args: Arguments) {
        error!("[{}]: {}", self.label, args);
    }
}

pub fn initial_logs(
    queue_name: &str,
    concurrency: usize,
    redis_url: &str,
    tasks_module_path: &str,
    tasks: &Vec<&String>,
) {
    info!("Queue: {}", queue_name);
    info!("Concurrency: {}", concurrency);
    info!("Redis: {}", redis_url);
    info!("Tasks module path: {}", tasks_module_path);
    info!("Tasks found: {:?}", tasks);
    info!("{}", "-".repeat(65));
}
