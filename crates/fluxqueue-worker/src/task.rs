use anyhow::{Context, Result, anyhow};
use fluxqueue_common::Task;
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

type Registry<T> = Arc<RwLock<HashMap<Arc<String>, T>>>;

#[derive(Debug)]
pub struct TaskData {
    func: Arc<Py<PyAny>>,
    context_name: Option<Arc<String>>,
}

#[derive(Debug)]
pub struct TaskRegistry {
    tasks: Registry<Arc<TaskData>>,
    contexts: Registry<Arc<Py<PyAny>>>,
    context_objects: Registry<Arc<Py<PyAny>>>,
}

impl TaskRegistry {
    pub fn new(module_path: &str, queue_name: &str) -> Result<Self> {
        let (tasks, contexts) = get_registry(module_path, queue_name)?;

        Ok(Self {
            tasks: Arc::new(RwLock::new(tasks)),
            contexts: Arc::new(RwLock::new(contexts)),
            context_objects: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn get_registered_tasks(&self) -> Result<Vec<String>> {
        let tasks = self.tasks.read().map_err(|e| anyhow!(e.to_string()))?;
        let task_names: Vec<_> = tasks.iter().map(|t| t.0.to_string()).collect();
        Ok(task_names)
    }

    pub fn get_registered_contexts(&self) -> Result<Vec<String>> {
        let contexts = self.contexts.read().map_err(|e| anyhow!(e.to_string()))?;
        let context_names: Vec<_> = contexts
            .iter()
            .filter(|(name, _)| name.as_str() != "_Context")
            .map(|(name, _)| name.to_string())
            .collect();
        Ok(context_names)
    }

    pub fn get_task(&self, name: Arc<String>) -> Option<Arc<TaskData>> {
        let tasks = self.tasks.read().ok()?;
        tasks.get(&name).cloned()
    }

    pub fn get_context(&self, name: Arc<String>) -> Option<Arc<Py<PyAny>>> {
        let contexts = self.contexts.read().ok()?;
        contexts.get(&name).cloned()
    }

    pub fn get_context_object(&self, name: Arc<String>) -> Option<Arc<Py<PyAny>>> {
        let ctx_objects = self.context_objects.read().ok()?;
        ctx_objects.get(&name).cloned()
    }

    pub fn get_task_context(&self, task_data: Arc<TaskData>) -> Result<Option<Arc<Py<PyAny>>>> {
        let Some(context_name) = task_data.context_name.clone() else {
            return Ok(None);
        };

        let context = self.get_context_object(context_name.clone());
        if let Some(context) = context {
            return Ok(Some(context.clone()));
        }

        let context_class = self.get_context(context_name.clone());

        if let Some(context_class) = context_class {
            Python::attach(|py| -> Result<Option<Arc<Py<PyAny>>>> {
                let context = context_class.call0(py)?;
                let context = Arc::new(context);

                py.detach(|| -> Result<()> {
                    let mut map = self.context_objects.write().map_err(|_| {
                        anyhow!(
                            "Internal Error: 'context_objects' lock poisoned (a thread panicked)"
                        )
                    })?;

                    map.insert(context_name.clone(), context.clone());
                    Ok(())
                })?;

                Ok(Some(context))
            })
        } else {
            Ok(None)
        }
    }
}

struct TaskRequest {
    task_registry: Arc<TaskRegistry>,
    executor_id: Arc<String>,
    task_data: Arc<TaskData>,
    task_name: Arc<String>,
    task: Arc<Task>,
    resp_tx: oneshot::Sender<Result<()>>,
}

pub struct PythonDispatcher {
    tx: mpsc::Sender<TaskRequest>,
    task_registry: Arc<TaskRegistry>,
}

impl PythonDispatcher {
    pub fn new(task_registry: Arc<TaskRegistry>) -> Result<Self> {
        let logical_cores = num_cpus::get();
        let (tx, mut rx) = mpsc::channel::<TaskRequest>(logical_cores * 2);

        let dispatcher = async move {
            while let Some(req) = rx.recv().await {
                run_task(
                    req.task_registry,
                    req.executor_id,
                    req.task_data,
                    req.task_name,
                    req.task,
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

        Ok(Self { tx, task_registry })
    }

    pub async fn execute(
        &self,
        executor_id: Arc<String>,
        task_data: Arc<TaskData>,
        task_name: Arc<String>,
        task: Arc<Task>,
    ) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(TaskRequest {
                task_registry: self.task_registry.clone(),
                executor_id,
                task_data,
                task_name,
                task,
                resp_tx,
            })
            .await
            .map_err(|_| anyhow!("Dispatcher channel closed"))?;

        resp_rx.await??;
        Ok(())
    }
}

struct MaybeCoro {
    func: Arc<Py<PyAny>>,
    args: Py<PyTuple>,
    kwargs: Py<PyDict>,
    context: Option<Arc<Py<PyAny>>>,
}

async fn run_task(
    task_registry: Arc<TaskRegistry>,
    executor_id: Arc<String>,
    task_data: Arc<TaskData>,
    task_name: Arc<String>,
    task: Arc<Task>,
) -> Result<()> {
    let logger = Logger::new(format!("EXECUTOR {}", &executor_id));
    let duration_start = Instant::now();

    let task_args: Value = from_slice(&task.args).context(format!(
        "Failed to deserialize task '{}' function args",
        &task_name
    ))?;
    let task_kwargs: Value = from_slice(&task.kwargs).context(format!(
        "Failed to deserialize task '{}' function kwargs",
        &task_name
    ))?;

    let context = task_registry.get_task_context(task_data.clone())?;

    let maybe_coro = Python::attach(|py| -> Result<Option<MaybeCoro>> {
        let py_args = pythonize(py, &task_args).context("Failed to pythonize args")?;
        let py_kwargs = pythonize(py, &task_kwargs).context("Failed to pythonize kwargs")?;

        let mut args_tuple = if let Ok(list) = py_args.cast::<PyList>() {
            list.to_tuple()
        } else if let Ok(tuple) = py_args.cast::<PyTuple>() {
            tuple.clone()
        } else {
            anyhow::bail!("Args must be an array/tuple, found {}", py_args.get_type());
        };

        if let Some(context) = context.as_ref() {
            let task_metadata = get_task_metadata(py, task.clone())?;
            let metadata_var = context.getattr(py, "_metadata_var")?;
            metadata_var.call_method1(py, "set", (task_metadata,))?;

            let context = context.as_any();
            let prefix = PyTuple::new(py, [context])?;

            let new_tuple = prefix.add(args_tuple.clone())?;
            if let Ok(tuple) = new_tuple.cast::<PyTuple>() {
                args_tuple = tuple.clone();
            }
        }

        let kwargs_dict = py_kwargs
            .cast_into::<PyDict>()
            .map_err(|_| anyhow!("Kwargs must be a map/dict"))?;

        let is_coroutine = is_coroutine(py, task_data.func.clone())?;

        if is_coroutine {
            Ok(Some(MaybeCoro {
                func: task_data.func.clone(),
                args: args_tuple.unbind(),
                kwargs: kwargs_dict.unbind(),
                context,
            }))
        } else {
            task_data
                .func
                .call(py, args_tuple.clone(), Some(&kwargs_dict))
                .map_err(|e| anyhow!("Failed to call Python function: {:?}", e))?;

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

    if let Some(maybe_coro) = maybe_coro {
        let fut = Python::attach(|py| {
            if let Some(context) = maybe_coro.context {
                let task_metadata = get_task_metadata(py, task.clone())
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                let result = context.call_method1(
                    py,
                    "_run_async_task",
                    (
                        maybe_coro.func.as_any(),
                        task_metadata,
                        maybe_coro.args,
                        Some(maybe_coro.kwargs),
                    ),
                )?;
                into_future(result.into_bound(py))
            } else {
                let result = maybe_coro
                    .func
                    .call(py, maybe_coro.args, Some(maybe_coro.kwargs.bind(py)))
                    .map_err(|e| anyhow!("Failed to call Python function: {:?}", e))
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

                into_future(result.into_bound(py))
            }
        })?;
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

type TasksAndContexts = (
    HashMap<Arc<String>, Arc<TaskData>>,
    HashMap<Arc<String>, Arc<Py<PyAny>>>,
);

fn get_registry(module_path: &str, queue_name: &str) -> Result<TasksAndContexts> {
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

    let result = Python::attach(|py| -> Result<TasksAndContexts> {
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

        let tasks: HashMap<Arc<String>, Arc<TaskData>> = registry
            .get_item("tasks")?
            .expect("tasks missing")
            .cast::<PyDict>()
            .map_err(|e| anyhow!("tasks is not a dict: {}", e))?
            .iter()
            .map(
                |(key, value)| -> Result<(Arc<String>, Arc<TaskData>), anyhow::Error> {
                    let name: String = key.extract()?;
                    let data = value.cast::<PyDict>().map_err(|e| anyhow!(e.to_string()))?;

                    let func = data
                        .get_item("func")?
                        .ok_or_else(|| anyhow!("Couldn't get the function"))?;
                    let func = Arc::new(func.unbind());

                    let context_name = data
                        .get_item("context_name")?
                        .map(|c| Arc::new(c.unbind().to_string()));

                    Ok((Arc::new(name), Arc::new(TaskData { func, context_name })))
                },
            )
            .collect::<Result<HashMap<_, _>, _>>()?;

        let mut contexts: HashMap<Arc<String>, Arc<Py<PyAny>>> = registry
            .get_item("contexts")?
            .expect("contexts missing")
            .cast::<PyDict>()
            .map_err(|e| anyhow!("contexts is not a dict: {}", e))?
            .iter()
            .filter_map(|(key, value): (Bound<PyAny>, Bound<PyAny>)| {
                let name: String = key.extract().ok()?;
                let class: Py<PyAny> = value.unbind();
                Some((Arc::new(name), Arc::new(class)))
            })
            .collect();

        let (base_name, base_class) = get_base_context_class(py)?;
        contexts.insert(base_name, base_class);

        Ok((tasks, contexts))
    })?;

    Ok(result)
}

fn get_task_metadata(py: Python<'_>, task: Arc<Task>) -> Result<Py<PyAny>> {
    let module = py.import("fluxqueue.models")?.unbind();
    let task_metadata = module.call_method1(
        py,
        "TaskMetadata",
        (
            task.id.clone(),
            task.retries,
            task.max_retries,
            task.created_at,
        ),
    )?;

    Ok(task_metadata)
}

fn get_base_context_class(py: Python<'_>) -> Result<(Arc<String>, Arc<Py<PyAny>>)> {
    let module = py.import("fluxqueue.context")?;
    let context_class = module.getattr("Context")?;
    Ok((
        Arc::new("_Context".to_string()),
        Arc::new(context_class.unbind()),
    ))
}

fn is_coroutine(py: Python<'_>, func: Arc<Py<PyAny>>) -> Result<bool> {
    let inspect = py.import("inspect")?;
    let is_coro: bool = inspect
        .call_method1("iscoroutinefunction", (func.as_any(),))?
        .extract()?;
    Ok(is_coro)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_module_path() -> Result<()> {
        let current_dir = Path::new("project");
        let tasks_path = Path::new("../project/tasks.py");
        let normalized_path = normalize_path(tasks_path);
        let module_path = path_to_module_path(current_dir, &normalized_path);
        let expected_path = Path::new("project/tasks.py");

        assert_eq!(normalized_path, expected_path);
        assert_eq!(module_path, Some("tasks".to_string()));

        Ok(())
    }

    fn get_test_module_path(filename: &str) -> String {
        let current_dir = std::env::current_dir().unwrap();
        let test_module_path = current_dir.join("tests").join(filename);
        test_module_path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_get_task_functions_valid_module() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let (tasks, _) = get_registry(&module_path_str, "default")?;

        assert_eq!(tasks.len(), 3);

        let task_names: Vec<Arc<String>> = tasks.iter().map(|(name, _)| name.clone()).collect();
        assert!(task_names.contains(&Arc::new("task-1".to_string())));
        assert!(task_names.contains(&Arc::new("task-2".to_string())));
        assert!(task_names.contains(&Arc::new("async-task".to_string())));

        assert!(!task_names.contains(&Arc::new("high-priority-task".to_string())));

        Ok(())
    }

    #[test]
    fn test_get_task_functions_different_queue() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let (tasks, _) = get_registry(&module_path_str, "high-priority")?;

        let task_names: Vec<Arc<String>> = tasks.iter().map(|(name, _)| name.clone()).collect();
        assert_eq!(tasks.len(), 1);
        assert!(task_names.contains(&Arc::new("high-priority-task".to_string())));

        Ok(())
    }

    #[test]
    fn test_get_task_functions_empty_module() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_empty.py");
        let (tasks, _) = get_registry(&module_path_str, "default")?;

        assert_eq!(tasks.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_task_functions_duplicate_names() {
        let module_path_str = get_test_module_path("test_tasks_duplicate.py");

        let result = get_registry(&module_path_str, "default");
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("duplicated") || error_msg.contains("duplicate"));
    }

    #[test]
    fn test_get_task_functions_invalid_path() {
        let result = get_registry("nonexistent/path/to/module.py", "default");
        assert!(result.is_err());
    }
}
