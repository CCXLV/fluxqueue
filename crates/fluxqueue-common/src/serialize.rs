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

pub fn deserialize_raw_task_data(raw_data: &Vec<u8>) -> Result<Task> {
    let task: Task = from_slice(raw_data).context("Failed to deserialize task data")?;
    Ok(task)
}

pub fn serialize_task_data(task: &Task) -> Result<Vec<u8>> {
    let blob = to_vec(task).context("Failed to serialize task data")?;
    Ok(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_task() -> Result<()> {
        let task = Task {
            id: "ID".to_string(),
            name: "test-task".to_string(),
            args: vec![],
            kwargs: vec![],
            created_at: 10,
            retries: 0,
            max_retries: 3,
        };

        let task_blob = serialize_task_data(&task)?;

        assert_eq!(task_blob.len(), 19);
        assert_eq!(task_blob[0], 151);

        Ok(())
    }

    #[test]
    fn test_deserialize_task() -> Result<()> {
        let task = Task {
            id: "ID".to_string(),
            name: "test-task".to_string(),
            args: vec![],
            kwargs: vec![],
            created_at: 10,
            retries: 0,
            max_retries: 3,
        };

        let task_blob = serialize_task_data(&task)?;

        let deserialized_task = deserialize_raw_task_data(&task_blob)?;

        assert_eq!(task.id, deserialized_task.id);
        assert_eq!(task.name, deserialized_task.name);
        assert_eq!(task.args, deserialized_task.args);
        assert_eq!(task.kwargs, deserialized_task.kwargs);
        assert_eq!(task.created_at, deserialized_task.created_at);
        assert_eq!(task.retries, deserialized_task.retries);
        assert_eq!(task.max_retries, deserialized_task.max_retries);

        Ok(())
    }
}
