use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use pyo3_async_runtimes::tokio::future_into_py;
use redis::aio::ConnectionManager;
use std::io::{Error, ErrorKind};

use fastqueue_worker;

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
        let redist_client = fastqueue_worker::get_redis_client(&redis_url)
            .map_err(|e| PyRuntimeError::new_err(e))?;
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
        let task_blob = fastqueue_worker::serialize_task(name, max_retries, args, kwargs)?;

        let mut conn = self.redis_client.clone();
        let _: () = redis::cmd("LPUSH")
            .arg(get_task_key(queue_name))
            .arg(task_blob)
            .query(&mut conn)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to push task to redis: {}", e),
                )
            })
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

        let task_blob = fastqueue_worker::serialize_task(name, max_retries, args, kwargs)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize task: {}", e)))?;

        let fut = async move {
            let mut conn_manager = ConnectionManager::new(client).await.map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e))
            })?;

            let _: () = redis::cmd("LPUSH")
                .arg(get_task_key(queue_name))
                .arg(task_blob)
                .query_async(&mut conn_manager)
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

fn get_task_key(queue_name: String) -> String {
    format!(
        "{}:{}",
        fastqueue_worker::redis_keys::TASK_QUEUE,
        queue_name
    )
}
