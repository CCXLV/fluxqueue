use std::fmt::Arguments;
#[cfg(test)]
use std::io::{self, Write};
#[cfg(test)]
use std::sync::{Arc, Mutex};
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
    info!("Tasks module: {}", tasks_module_path);
    info!("Tasks found: {:?}", tasks);
    info!("Starting up the executors...");
}

#[cfg(test)]
pub struct TestWriter {
    pub logs: Arc<Mutex<Vec<String>>>,
}

#[cfg(test)]
impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let line = String::from_utf8_lossy(buf).to_string();
        self.logs.lock().unwrap().push(line);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
