use pyo3::types::{PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::time::{SystemTime, UNIX_EPOCH};

use fastqueue_worker;

#[pyclass(subclass)]
pub struct FastQueueCore {
    pub(crate) redis_url: String,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(signature = (redis_url="redis://127.0.0.1:6379".to_string()))]
    fn new(redis_url: String) -> Self {
        Self { redis_url }
    }

    async fn register_task(&self, name: String, module_path: String) -> PyResult<()> {
        let redis_client = fastqueue_worker::RedisClient::new(&self.redis_url, None)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e)))?;
        redis_client
            .register_task(name, module_path)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to register the task: {}", e)))?;

        Ok(())
    }

    async fn enqueue(
        &self,
        name: String,
        args: Py<PyTuple>,
        kwargs: Option<Py<PyDict>>,
    ) -> PyResult<()> {
        let arg_bytes = Python::attach(|py| {
            let bound_args = args.into_bound(py).into_any();
            fastqueue_worker::serialize_python_to_msgpack(bound_args)
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let kwarg_bytes = if let Some(kwargs) = kwargs {
            Python::attach(|py| {
                let bound_kwargs = kwargs.into_bound(py).into_any();
                fastqueue_worker::serialize_python_to_msgpack(bound_kwargs)
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
        } else {
            vec![128]
        };

        let redis_client = fastqueue_worker::RedisClient::new(&self.redis_url, None)
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect to Redis: {}", e)))?;

        let task = fastqueue_worker::Task {
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

        let task_blob = fastqueue_worker::serialize_task_data(&task)?;

        let _ = redis_client
            .push_task(task_blob)
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
