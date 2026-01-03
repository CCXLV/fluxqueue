use pyo3::prelude::*;

mod queue;
mod runtime;

use queue::QueueHandle;

#[pyclass(subclass)]
pub struct FastQueueCore {
    inner: QueueHandle,
}

#[pymethods]
impl FastQueueCore {
    #[new]
    #[pyo3(signature = (max_workers=10))]
    fn new(max_workers: usize) -> Self {
        let inner = runtime::start_runtime(max_workers);
        FastQueueCore { inner }
    }

    fn enqueue(&self, func: Py<PyAny>) {
        self.inner.enqueue(func);
    }

    fn _shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.shutdown().await;
            Ok(())
        })
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
