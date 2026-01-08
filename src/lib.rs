use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};

use fastqueue_worker;

mod redis_client;

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) redis_url: String,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(
        signature = (
            redis_url = "redis://127.0.0.1:6379".to_string(),
        )
    )]
    fn new(redis_url: String) -> Self {
        Self { redis_url }
    }

    fn _enqueue(
        &self,
        name: String,
        queue_name: String,
        max_retries: u8,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<()> {
        let redis_client = redis_client::RedisClient::new(&self.redis_url)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e)))?;

        let task_blob = fastqueue_worker::serialize_task(name, max_retries, args, kwargs)?;

        let _ = redis_client
            .push_task(queue_name, task_blob)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(())
    }

    async fn _enqueue_async(
        &self,
        name: String,
        queue_name: String,
        max_retries: u8,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<()> {
        let redis_client = fastqueue_worker::RedisClient::new(&self.redis_url, None)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e)))?;

        let task_blob = fastqueue_worker::serialize_task(name, max_retries, args, kwargs)?;

        let _ = redis_client
            .push_task(queue_name, task_blob)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(())
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
