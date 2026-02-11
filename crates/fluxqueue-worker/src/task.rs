use anyhow::{Result, anyhow};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::into_future;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

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
}

type PyResponse = Pin<Box<dyn Future<Output = PyResult<Py<PyAny>>> + Send>>;

struct TaskRequest {
    func: Py<PyAny>,
    resp_tx: oneshot::Sender<PyResult<PyResponse>>,
}

pub struct PythonDispatcher {
    tx: mpsc::Sender<TaskRequest>,
}

impl PythonDispatcher {
    pub fn new() -> Self {
        let logical_cores = num_cpus::get();
        let (tx, mut rx) = mpsc::channel::<TaskRequest>(logical_cores * 2);

        std::thread::spawn(move || {
            Python::attach(|py| {
                let dispatcher = async move {
                    while let Some(req) = rx.recv().await {
                        let res = Python::attach(|py| into_future(req.func.into_bound(py)));

                        let _ = req.resp_tx.send(res.map(|f| Box::pin(f) as PyResponse));
                    }
                    Ok(())
                };

                pyo3_async_runtimes::tokio::run(py, dispatcher).expect("Python loop failed");
            });
        });

        Self { tx }
    }

    pub async fn execute(&self, func: Py<PyAny>) -> Result<Py<PyAny>> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(TaskRequest { func, resp_tx })
            .await
            .map_err(|_| anyhow!("Dispatcher channel closed"))?;

        let py_fut = resp_rx.await??;

        let result = py_fut.await?;
        Ok(result)
    }
}
