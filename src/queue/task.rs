use pyo3::prelude::*;

pub struct Task {
    pub func: Py<PyAny>,
}
