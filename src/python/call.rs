use pyo3::prelude::*;

pub fn call_python(func: Py<PyAny>) {
    Python::attach(|py| {
        if let Err(e) = func.call0(py) {
            e.print(py);
        }
    });
}
