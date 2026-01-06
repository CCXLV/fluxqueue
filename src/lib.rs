use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod queue;
mod redis_client;
mod runtime;

use fastqueue_common;
use queue::QueueHandle;

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) inner: QueueHandle,
    pub(crate) registry: fastqueue_common::TaskRegistry,
    pub(crate) redis_client: redis::Client,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(signature = (redis_url="redis://127.0.0.1:6379".to_string()))]
    fn new(redis_url: String) -> PyResult<Self> {
        let inner = runtime::start_runtime(4);
        let redis_client = fastqueue_common::get_redis_client(&redis_url)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e)))?;

        Ok(FastQueueCore {
            inner,
            registry: fastqueue_common::TaskRegistry {
                tasks: Arc::new(RwLock::new(HashMap::new())),
            },
            redis_client,
        })
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
        let arg_bytes = fastqueue_common::serialize_python_to_msgpack(bound_args)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let kwarg_bytes = if let Some(kwargs) = kwargs {
            let bound_kwargs = kwargs.into_bound(py).into_any();
            fastqueue_common::serialize_python_to_msgpack(bound_kwargs)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
        } else {
            vec![128] // 128: Msgpack Decimal for empty dictionary
        };

        let task = fastqueue_common::Task {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            args: arg_bytes,
            kwargs: kwarg_bytes,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            retries: 0,
        };

        let _ = redis_client::push_task(&self.redis_client, task)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

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
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
