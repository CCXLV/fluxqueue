use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use pyo3_async_runtimes::tokio::future_into_py;
use redis::aio::ConnectionManager;

use fastqueue_common::{get_redis_client, push_task, push_task_async, serialize_task};

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) redis_client: redis::Client,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(
        signature = (
            redis_url = "redis://127.0.0.1:6379".to_string(),
        )
    )]
    fn new(redis_url: String) -> PyResult<Self> {
        let redist_client =
            get_redis_client(&redis_url).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            redis_client: redist_client,
        })
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

        let mut conn = self.redis_client.clone();
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
        let client = self.redis_client.clone();

        let task_blob = serialize_task(name, max_retries, args, kwargs)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize task: {}", e)))?;

        let fut = async move {
            let mut conn_manager = ConnectionManager::new(client).await.map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e))
            })?;

            push_task_async(&mut conn_manager, queue_name, task_blob)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            Ok(())
        };

        future_into_py(py, fut)
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
