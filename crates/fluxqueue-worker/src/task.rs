use anyhow::{Context, Result, anyhow};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyModule, PyTuple};
use pyo3_async_runtimes::tokio::into_future;
use pythonize::pythonize;
use rmp_serde::from_slice;
use rmpv::Value;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use crate::logger::Logger;

type TasksAndContexts = (
    HashMap<String, Arc<Py<PyAny>>>,
    HashMap<String, Arc<Py<PyAny>>>,
);

#[derive(Debug)]
pub struct TaskRegistry {
    tasks: Arc<RwLock<HashMap<String, Arc<Py<PyAny>>>>>,
    contexts: Arc<RwLock<HashMap<String, Arc<Py<PyAny>>>>>,
}

impl TaskRegistry {
    pub fn new(module_path: &str, queue_name: &str) -> Result<Self> {
        let script = include_str!("../scripts/get_registry.py");
        let script_cstr = CString::new(script)?;
        let filename = CString::new("get_registry.py")?;
        let module_name = CString::new("get_registry")?;

        let full_current_dir = std::env::current_dir().unwrap();
        let full_module_path = full_current_dir.join(module_path);
        let clean_module_path = normalize_path(&full_module_path);
        let project_root = full_current_dir
            .ancestors()
            .find(|p| p.join("tests").exists())
            .unwrap_or(&full_current_dir);
        let real_module_path = path_to_module_path(project_root, &clean_module_path);

        if !clean_module_path.exists() || real_module_path.is_none() {
            return Err(anyhow!(
                "Tasks module path {:?} doesn't exist.",
                clean_module_path
            ));
        }

        let real_module_path = real_module_path.unwrap();
        let module_dir = project_root.to_string_lossy().to_string();

        let (tasks, contexts) = Python::attach(|py| -> Result<TasksAndContexts> {
            let module = PyModule::from_code(
                py,
                script_cstr.as_c_str(),
                filename.as_c_str(),
                module_name.as_c_str(),
            )
            .map_err(|e| anyhow!("Failed to import python module: {}", e))?;

            let registry: Bound<'_, PyDict> = module
                .getattr("get_registry")
                .map_err(|e| anyhow!("Failed to get 'get_registry' script: {}", e))?
                .call1((real_module_path, queue_name, module_dir))
                .map_err(|e| anyhow!("Failed to get tasks: {}", e))?
                .cast_into::<PyDict>()
                .map_err(|_| anyhow!("Failed to cast result to a Python Dictionary"))?;

            let tasks: HashMap<String, Arc<Py<PyAny>>> = registry
                .get_item("tasks")?
                .expect("tasks missing")
                .cast::<PyDict>()
                .map_err(|e| anyhow!("tasks is not a dict: {}", e))?
                .iter()
                .filter_map(|(key, value)| {
                    let name: String = key.extract().ok()?;
                    let func: Py<PyAny> = value.unbind();
                    Some((name, Arc::new(func)))
                })
                .collect();

            let contexts: HashMap<String, Arc<Py<PyAny>>> = registry
                .get_item("contexts")?
                .expect("contexts missing")
                .cast::<PyDict>()
                .map_err(|e| anyhow!("contexts is not a dict: {}", e))?
                .iter()
                .filter_map(|(key, value): (Bound<PyAny>, Bound<PyAny>)| {
                    let name: String = key.extract().ok()?;
                    let func: Py<PyAny> = value.unbind();
                    Some((name, Arc::new(func)))
                })
                .collect();

            Ok((tasks, contexts))
        })?;

        Ok(Self {
            tasks: Arc::new(RwLock::new(tasks)),
            contexts: Arc::new(RwLock::new(contexts)),
        })
    }

    pub fn get_registered_tasks(&self) -> Result<Vec<String>> {
        let tasks = self.tasks.read().map_err(|e| anyhow!(e.to_string()))?;
        let task_names: Vec<_> = tasks.iter().map(|t| t.0.to_string()).collect();
        Ok(task_names)
    }

    pub fn get_registered_contexts(&self) -> Result<Vec<String>> {
        let contexts = self.contexts.read().map_err(|e| anyhow!(e.to_string()))?;
        let context_names: Vec<_> = contexts.iter().map(|t| t.0.to_string()).collect();
        Ok(context_names)
    }

    pub fn get(&self, name: &str) -> Option<Arc<Py<PyAny>>> {
        let tasks = self.tasks.read().ok()?;
        tasks.get(name).cloned()
    }
}

struct TaskRequest {
    executor_id: Arc<String>,
    func: Arc<Py<PyAny>>,
    task_name: Arc<String>,
    raw_args: Arc<Vec<u8>>,
    raw_kwargs: Arc<Vec<u8>>,
    resp_tx: oneshot::Sender<Result<()>>,
}

pub struct PythonDispatcher {
    tx: mpsc::Sender<TaskRequest>,
}

impl PythonDispatcher {
    pub fn new() -> Result<Self> {
        let logical_cores = num_cpus::get();
        let (tx, mut rx) = mpsc::channel::<TaskRequest>(logical_cores * 2);

        let dispatcher = async move {
            while let Some(req) = rx.recv().await {
                run_task(
                    req.executor_id,
                    req.func,
                    req.task_name,
                    req.raw_args,
                    req.raw_kwargs,
                )
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

                let _ = req.resp_tx.send(Ok(()));
            }
            Ok(())
        };

        tokio::task::spawn_blocking(move || {
            Python::attach(|py| {
                pyo3_async_runtimes::tokio::run(py, dispatcher).expect("Python loop failed");
            });
        });

        Ok(Self { tx })
    }

    pub async fn execute(
        &self,
        executor_id: Arc<String>,
        func: Arc<Py<PyAny>>,
        task_name: Arc<String>,
        raw_args: Arc<Vec<u8>>,
        raw_kwargs: Arc<Vec<u8>>,
    ) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(TaskRequest {
                executor_id,
                func,
                task_name,
                raw_args,
                raw_kwargs,
                resp_tx,
            })
            .await
            .map_err(|_| anyhow!("Dispatcher channel closed"))?;

        resp_rx.await??;
        Ok(())
    }
}

async fn run_task(
    executor_id: Arc<String>,
    task_function: Arc<Py<PyAny>>,
    task_name: Arc<String>,
    raw_args: Arc<Vec<u8>>,
    raw_kwargs: Arc<Vec<u8>>,
) -> Result<()> {
    let logger = Logger::new(format!("EXECUTOR {}", &executor_id));
    let duration_start = Instant::now();

    let task_args: Value = from_slice(&raw_args).context(format!(
        "Failed to deserialize task '{}' function args",
        &task_name
    ))?;
    let task_kwargs: Value = from_slice(&raw_kwargs).context(format!(
        "Failed to deserialize task '{}' function kwargs",
        &task_name
    ))?;

    let maybe_coro = Python::attach(|py| -> Result<Option<Py<PyAny>>> {
        let py_args = pythonize(py, &task_args).context("Failed to pythonize args")?;
        let py_kwargs = pythonize(py, &task_kwargs).context("Failed to pythonize kwargs")?;

        let args_tuple = if let Ok(list) = py_args.cast::<PyList>() {
            list.to_tuple()
        } else if let Ok(tuple) = py_args.cast::<PyTuple>() {
            tuple.clone()
        } else {
            anyhow::bail!("Args must be an array/tuple, found {}", py_args.get_type());
        };

        let kwargs_dict = py_kwargs
            .cast_into::<PyDict>()
            .map_err(|_| anyhow!("Kwargs must be a map/dict"))?;

        let result = task_function
            .call(py, args_tuple, Some(&kwargs_dict))
            .map_err(|e| anyhow!("Failed to call Python function: {:?}", e))?;

        let bound_result = result.bind(py);
        let is_coroutine = bound_result
            .hasattr("__await__")
            .map_err(|_| anyhow!("Failed to check if result is awaitable"))?;

        if is_coroutine {
            Ok(Some(result))
        } else {
            let duration_end = duration_start.elapsed();
            logger.info(format_args!(
                "Task '{}' successfully finished in {}ms",
                &task_name,
                duration_end.as_millis()
            ));

            Ok(None)
        }
    })
    .map_err(|e| {
        let duration_end = duration_start.elapsed();
        logger.error(format_args!(
            "Task '{}' failed in {}ms: {}",
            &task_name,
            duration_end.as_millis(),
            e
        ));
        anyhow!(e.to_string())
    })?;

    if let Some(coro) = maybe_coro {
        let fut = Python::attach(|py| into_future(coro.into_bound(py)))?;
        fut.await?;

        let duration_end = duration_start.elapsed();
        logger.info(format_args!(
            "Task '{}' successfully finished in {}ms",
            &task_name,
            duration_end.as_millis()
        ));
    }

    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

fn path_to_module_path(current_dir: &Path, target_path: &Path) -> Option<String> {
    let rel_path = target_path.strip_prefix(current_dir).ok()?;

    let mut components: Vec<String> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if let Some(last) = components.last_mut()
        && let Some(pos) = last.rfind('.')
    {
        last.truncate(pos);
    }

    Some(components.join("."))
}
