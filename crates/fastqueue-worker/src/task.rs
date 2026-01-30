use anyhow::Result;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct TaskRegistry {
    tasks: Arc<RwLock<HashMap<String, Arc<Py<PyAny>>>>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, name: String, func: Py<PyAny>) -> Result<()> {
        let mut tasks = self.tasks.write().map_err(|_| {
            anyhow::anyhow!("Internal Error: Task registry lock poisoned (a thread panicked)")
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
