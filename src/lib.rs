use pyo3::prelude::*;

mod python;
mod queue;
mod runtime;

use queue::QueueHandle;

#[pyclass]
pub struct TaskQueue {
    inner: QueueHandle,
}

#[pymethods]
impl TaskQueue {
    #[new]
    #[pyo3(signature = (max_workers=10))]
    fn new(py: Python<'_>, max_workers: usize) -> Self {
        let inner = runtime::start_runtime(py, max_workers);
        TaskQueue { inner }
    }
    
    fn enqueue(&self, func: Py<PyAny>) {
        self.inner.enqueue(func);
    }
    
    fn wait(&self, py: Python<'_>) {
        py.detach(|| {
            self.inner.shutdown_and_wait();
        });
    }
}

#[pymodule]
fn fastqueue_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TaskQueue>()?;
    Ok(())
}
