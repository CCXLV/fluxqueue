use log::info;
use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod queue;
mod runtime;

use fastqueue_worker;
use fastqueue_common;
use queue::QueueHandle;

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) inner: QueueHandle,
    pub(crate) registry: fastqueue_common::TaskRegistry,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(signature = (workers=10))]
    fn new(workers: usize) -> Self {
        let inner = runtime::start_runtime(workers);
        FastQueueCore {
            inner,
            registry: fastqueue_common::TaskRegistry {
                tasks: Arc::new(RwLock::new(HashMap::new())),
            },
        }
    }

    fn register_task(&self, name: String, func: Py<PyAny>) -> PyResult<()> {
        fastqueue_common::register_task(name, func, &self.registry)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    fn enqueue(
        &self,
        py: Python<'_>,
        name: String,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<()> {
        let bound_args = args.into_bound(py).into_any();
        let arg_bytes = fastqueue_common::deserialize_python_to_msgpack(bound_args)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        info!("{:?}", arg_bytes);

        Ok(())
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
    pyo3_log::init();
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
