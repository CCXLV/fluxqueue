use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod queue;
mod runtime;

use fastqueue_worker;
use queue::QueueHandle;

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) inner: QueueHandle,
    pub(crate) registry: fastqueue_worker::TaskRegistry,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(signature = (workers=10))]
    fn new(workers: usize) -> Self {
        let inner = runtime::start_runtime(workers);
        FastQueueCore {
            inner,
            registry: fastqueue_worker::TaskRegistry {
                tasks: Arc::new(RwLock::new(HashMap::new())),
            },
        }
    }

    fn register_task(&self, name: String, func: Py<PyAny>) -> PyResult<()> {
        fastqueue_worker::register_task(name, func, &self.registry)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    fn enqueue(&self, py: Python<'_>, name: String, args: Py<PyTuple>, kwargs: Option<Py<PyDict>>) {
    }

    fn shutdown(&self, py: Python<'_>) {
        py.detach(|| {
            self.inner.shutdown();
        });
    }

    fn async_shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.async_shutdown().await;
            Ok(())
        })
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
