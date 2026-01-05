use pyo3::prelude::*;
use rmp_serde::to_vec_named;
use rmpv::Value;
use serde_pyobject::from_pyobject;
use std::io::{Error, ErrorKind};

pub fn deserialize_python_to_msgpack(object: Bound<'_, PyAny>) -> Result<Vec<u8>, Error> {
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
