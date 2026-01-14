use anyhow::{Context, Result};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use rmp_serde::{from_slice, to_vec, to_vec_named};
use rmpv::Value;
use serde_pyobject::from_pyobject;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::task::Task;

pub fn serialize_task(
    name: String,
    max_retries: u8,
    args: Py<PyTuple>,
    kwargs: Option<Py<PyDict>>,
) -> Result<Vec<u8>> {
    let arg_bytes = Python::attach(|py| {
        let bound_args = args.into_bound(py).into_any();
        serialize_python_to_msgpack(bound_args)
    })?;

    let kwarg_bytes = if let Some(kwargs) = kwargs {
        Python::attach(|py| {
            let bound_kwargs = kwargs.into_bound(py).into_any();
            serialize_python_to_msgpack(bound_kwargs)
        })?
    } else {
        vec![128]
    };

    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        args: arg_bytes,
        kwargs: kwarg_bytes,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        retries: 0,
        max_retries,
    };

    let task_blob = serialize_task_data(&task)?;

    Ok(task_blob)
}

pub fn serialize_python_to_msgpack(object: Bound<'_, PyAny>) -> Result<Vec<u8>> {
    let serialized_obj: Value =
        from_pyobject(object).context("Failed to convert Python object to msgpack Value")?;

    let bytes = to_vec_named(&serialized_obj).context("msgpack serialization failed")?;

    Ok(bytes)
}

pub fn deserialize_raw_task_data(raw_data: Vec<u8>) -> Result<Task> {
    let task: Task = from_slice(&raw_data).context("Failed to deserialize task data")?;
    Ok(task)
}

fn serialize_task_data(task: &Task) -> Result<Vec<u8>> {
    let blob = to_vec(task).context("Failed to serialize task data")?;
    Ok(blob)
}
