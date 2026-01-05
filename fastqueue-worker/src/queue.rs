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
    pub timestamp: i64,
    pub retries: u32,
}

pub struct TaskRegistry {
    // We use RwLock so many workers can READ at once,
    // but we can WRITE to it during registration.
    pub tasks: Arc<RwLock<HashMap<String, Py<PyAny>>>>,
}

pub fn register_task(name: String, func: Py<PyAny>, registry: &TaskRegistry) -> Result<(), Error> {
    let mut tasks = registry.tasks.write().map_err(|_| {
        Error::new(
            ErrorKind::Other,
            "Internal Error: Task registry lock poisoned (a thread panicked)",
        )
    })?;
    tasks.insert(name, func);

    Ok(())
}
