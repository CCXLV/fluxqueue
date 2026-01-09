use pyo3::prelude::*;
use rmp_serde::{to_vec, to_vec_named};
use rmpv::Value;
use serde_pyobject::from_pyobject;
use std::io::{Error, ErrorKind};

use crate::task::Task;

pub fn serialize_python_to_msgpack(object: Bound<'_, PyAny>) -> Result<Vec<u8>, Error> {
    let serialized_obj: Value =
        from_pyobject(object).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let bytes = to_vec_named(&serialized_obj).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("msgpack serializaion failed: {}", e),
        )
    })?;

    Ok(bytes)
}

pub fn serialize_task_data(task: &Task) -> Result<Vec<u8>, Error> {
    let blob = to_vec(task).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to serialize task data: {}", e),
        )
    })?;
    Ok(blob)
}

pub fn deserialize_raw_task_data(raw_data: Vec<u8>) -> Result<Task, Error> {
    let task: Task = rmp_serde::from_slice(&raw_data).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to deserialize task data: {}", e),
        )
    })?;
    Ok(task)
}
