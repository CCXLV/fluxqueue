use anyhow::{Context, Result, anyhow};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3_async_runtimes::tokio::into_future;
use pythonize::pythonize;
use rmp_serde::from_slice;
use rmpv::Value;
use std::collections::HashMap;
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
            anyhow!("Internal Error: Task registry lock poisoned (a thread panicked)")
        })?;
        tasks.insert(name, Arc::new(func));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<Py<PyAny>>> {
        let tasks = self.tasks.read().ok()?;
        tasks.get(name).cloned()
    }
}

struct TaskRequest {
    func: Arc<Py<PyAny>>,
    task_name: Arc<String>,
    raw_args: Arc<Vec<u8>>,
    raw_kwargs: Arc<Vec<u8>>,
    resp_tx: oneshot::Sender<Result<()>>,
}

pub struct PythonDispatcher {
    tx: mpsc::Sender<TaskRequest>,
}

impl PythonDispatcher {
    pub fn new() -> Result<Self> {
        let logical_cores = num_cpus::get();
        let (tx, mut rx) = mpsc::channel::<TaskRequest>(logical_cores * 2);

        let dispatcher = async move {
            while let Some(req) = rx.recv().await {
                run_task(req.func, req.task_name, req.raw_args, req.raw_kwargs)
                    .await
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

                let _ = req.resp_tx.send(Ok(()));
            }
            Ok(())
        };

        tokio::task::spawn_blocking(move || {
            Python::attach(|py| {
                pyo3_async_runtimes::tokio::run(py, dispatcher).expect("Python loop failed");
            });
        });

        Ok(Self { tx })
    }

    pub async fn execute(
        &self,
        func: Arc<Py<PyAny>>,
        task_name: Arc<String>,
        raw_args: Arc<Vec<u8>>,
        raw_kwargs: Arc<Vec<u8>>,
    ) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(TaskRequest {
                func,
                task_name,
                raw_args,
                raw_kwargs,
                resp_tx,
            })
            .await
            .map_err(|_| anyhow!("Dispatcher channel closed"))?;

        resp_rx.await??;
        Ok(())
    }
}

async fn run_task(
    task_function: Arc<Py<PyAny>>,
    task_name: Arc<String>,
    raw_args: Arc<Vec<u8>>,
    raw_kwargs: Arc<Vec<u8>>,
) -> Result<()> {
    let task_args: Value = from_slice(&raw_args).context(format!(
        "Failed to deserialize task '{}' function args",
        &task_name
    ))?;
    let task_kwargs: Value = from_slice(&raw_kwargs).context(format!(
        "Failed to deserialize task '{}' function kwargs",
        &task_name
    ))?;

    let maybe_coro = Python::attach(|py| -> Result<Option<Py<PyAny>>> {
        let py_args = pythonize(py, &task_args).context("Failed to pythonize args")?;
        let py_kwargs = pythonize(py, &task_kwargs).context("Failed to pythonize kwargs")?;

        let args_tuple = if let Ok(list) = py_args.cast::<PyList>() {
            list.to_tuple()
        } else if let Ok(tuple) = py_args.cast::<PyTuple>() {
            tuple.clone()
        } else {
            anyhow::bail!("Args must be an array/tuple, found {}", py_args.get_type());
        };

        let kwargs_dict = py_kwargs
            .cast_into::<PyDict>()
            .map_err(|_| anyhow!("Kwargs must be a map/dict"))?;

        let result = task_function
            .call(py, args_tuple, Some(&kwargs_dict))
            .map_err(|e| anyhow!("Failed to call Python function: {:?}", e))?;

        let bound_result = result.bind(py);
        let is_coroutine = bound_result
            .hasattr("__await__")
            .map_err(|_| anyhow!("Failed to check if result is awaitable"))?;

        if is_coroutine {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    })?;

    if let Some(coro) = maybe_coro {
        let fut = Python::attach(|py| into_future(coro.into_bound(py)))?;
        fut.await?;
    }

    Ok(())
}
