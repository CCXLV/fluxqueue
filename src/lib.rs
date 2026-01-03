use pyo3::prelude::*;

mod python;
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
    fn new(py: Python<'_>, max_workers: usize) -> Self {
        let inner = runtime::start_runtime(py, max_workers);
        FastQueueCore { inner }
    }

    fn enqueue(&self, func: Py<PyAny>) {
        self.inner.enqueue(func);
    }

    fn shutdown(&self, py: Python<'_>) {
        py.detach(|| {
            self.inner.shutdown();
        });
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FastQueueCore>()?;
    Ok(())
}
