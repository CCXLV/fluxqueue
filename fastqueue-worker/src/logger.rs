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
