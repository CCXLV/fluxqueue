use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
    pub created_at: u64,
    pub retries: u8,
    pub max_retries: u8,
}

pub struct TaskRegistry {
    // We use RwLock so many workers can READ at once,
    // but we can WRITE to it during registration.
    tasks: Arc<RwLock<HashMap<String, Arc<Py<PyAny>>>>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, name: String, func: Py<PyAny>) -> Result<(), Error> {
        let mut tasks = self.tasks.write().map_err(|_| {
            Error::new(
                ErrorKind::Other,
                "Internal Error: Task registry lock poisoned (a thread panicked)",
            )
        })?;
        tasks.insert(name, Arc::new(func));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<Py<PyAny>>> {
        let tasks = self.tasks.read().ok()?;
        tasks.get(name).cloned()
    }

    pub fn remove(&self, name: &str) {
        if let Ok(mut tasks) = self.tasks.write() {
            tasks.remove(name);
        }
    }
}
