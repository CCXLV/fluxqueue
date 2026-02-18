use deadpool_redis::{Config, Runtime};
use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use pyo3_async_runtimes::tokio::future_into_py;

use fluxqueue_common::{get_redis_connection, push_task, push_task_async, serialize_task};

#[pyclass(subclass)]
pub struct FluxQueueCore {
    pub(crate) redis_url: String,
}

#[pymethods]
impl FluxQueueCore {
    #[new]
    #[pyo3(
        signature = (
            redis_url = "redis://127.0.0.1:6379".to_string(),
        )
    )]
    fn new(redis_url: String) -> PyResult<Self> {
        Ok(Self { redis_url })
    }

    fn _enqueue(
        &self,
        name: String,
        queue_name: String,
        max_retries: u8,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<()> {
        let task_blob = serialize_task(name, max_retries, args, kwargs)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let mut conn = get_redis_connection(&self.redis_url)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        push_task(&mut conn, queue_name, task_blob)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(())
    }

    fn _enqueue_async<'py>(
        &self,
        py: Python<'py>,
        name: String,
        queue_name: String,
        max_retries: u8,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let redis_url = self.redis_url.clone();

        let task_blob = serialize_task(name, max_retries, args, kwargs)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize task: {}", e)))?;

        let fut = async move {
            let redis_config = Config::from_url(redis_url);
            let redis_pool = redis_config.create_pool(Some(Runtime::Tokio1)).unwrap();
            let mut conn_manager = redis_pool
                .get()
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            push_task_async(&mut conn_manager, queue_name, task_blob)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            Ok(())
        };

        future_into_py(py, fut)
    }
}

#[pymodule]
fn fluxqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FluxQueueCore>()?;
    Ok(())
}
