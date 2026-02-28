use anyhow::{Result, anyhow};
use pyo3::Python;
use pyo3::types::PyAnyMethods;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::logger::{Logger, initial_logs};
use crate::redis_client::RedisClient;
use crate::task::{PythonDispatcher, TaskData, TaskRegistry};
use fluxqueue_common::{Task, deserialize_raw_task_data};

static MIN_CLIENT_LIB_VERSION: &str = "0.3.0";

pub async fn run_worker(
    mut shutdown: watch::Receiver<bool>,
    concurrency: usize,
    redis_url: String,
    tasks_module_path: String,
    queue_name: String,
    save_dead_tasks: bool,
) -> Result<()> {
    check_client_library_version().map_err(|e| {
        tracing::error!("{}", e);
        std::process::exit(1);
    })?;

    let redis_client = RedisClient::new(&redis_url).await.map_err(|e| {
        tracing::error!("{}", e);
        std::process::exit(1);
    })?;
    let redis_client = Arc::new(redis_client);

    let task_registry = Arc::new(TaskRegistry::new(&tasks_module_path, &queue_name).map_err(
        |e| {
            tracing::error!("{}", e);
            std::process::exit(1);
        },
    )?);
    let registered_tasks = task_registry.get_registered_tasks()?;
    let registered_contexts = task_registry.get_registered_contexts()?;

    initial_logs(
        &queue_name,
        concurrency,
        &redis_url,
        &tasks_module_path,
        registered_tasks,
        registered_contexts,
    );

    let queue_name = Arc::new(queue_name);
    let executor_ids = generate_executor_ids(concurrency);
    let atomic_concurrency = Arc::new(AtomicUsize::new(concurrency));
    let started_executors_count = Arc::new(AtomicUsize::new(0));
    let mut executors = JoinSet::new();

    let executor_futures: Vec<_> = (0..concurrency)
        .map(|i| {
            let shutdown = shutdown.clone();
            let concurrency = Arc::clone(&atomic_concurrency);
            let counter = Arc::clone(&started_executors_count);
            let redis_client = Arc::clone(&redis_client);
            let queue_name = Arc::clone(&queue_name);
            let executor_id = Arc::clone(&executor_ids[i]);
            let task_registry = Arc::clone(&task_registry);

            async move {
                let python_dispatcher = Arc::new(PythonDispatcher::new(task_registry.clone())?);

                redis_client
                    .register_executor(&queue_name, &executor_id)
                    .await?;

                redis_client.set_executor_heartbeat(&executor_id).await?;

                let ready_check = ReadyCheck {
                    concurrency,
                    counter,
                };

                let executor_context = ExecutorContext {
                    queue_name,
                    executor_id,
                    redis_client,
                    task_registry,
                    python_dispatcher,
                };

                Ok::<_, anyhow::Error>((shutdown, ready_check, executor_context))
            }
        })
        .collect();

    let results = futures::future::try_join_all(executor_futures).await?;
    for result in results {
        let (shutdown, ready_check, executor_context) = result;
        executors.spawn(executor_loop(shutdown, ready_check, executor_context));
    }

    let janitor_shutdown = shutdown.clone();
    let concurrency = Arc::clone(&atomic_concurrency);
    let counter = Arc::clone(&started_executors_count);
    let janitor_queue_name = Arc::clone(&queue_name);
    let janitor_redis = Arc::clone(&redis_client);
    let janitor_executor_ids = Arc::clone(&executor_ids);
    let save_dead_tasks = Arc::new(save_dead_tasks);

    let ready_check = ReadyCheck {
        concurrency,
        counter,
    };

    executors.spawn(janitor_loop(
        janitor_shutdown,
        ready_check,
        janitor_queue_name,
        janitor_executor_ids,
        save_dead_tasks,
        janitor_redis,
    ));

    shutdown.changed().await.ok();

    while let Some(res) = executors.join_next().await {
        if let Err(e) = res {
            tracing::error!("Worker panicked: {}", e);
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    redis_client
        .cleanup_executors_registry(&queue_name, executor_ids)
        .await?;

    Ok(())
}

struct ExecutorContext {
    queue_name: Arc<String>,
    executor_id: Arc<String>,
    redis_client: Arc<RedisClient>,
    task_registry: Arc<TaskRegistry>,
    python_dispatcher: Arc<PythonDispatcher>,
}

struct ReadyCheck {
    concurrency: Arc<AtomicUsize>,
    counter: Arc<AtomicUsize>,
}

async fn executor_loop(
    mut shutdown: watch::Receiver<bool>,
    ready_check: ReadyCheck,
    ctx: ExecutorContext,
) -> Result<()> {
    let logger = Logger::new(format!("EXECUTOR {}", &ctx.executor_id));

    ready_check.counter.fetch_add(1, Ordering::SeqCst);
    check_worker_is_ready(ready_check);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                return Ok(())
            }

            res = ctx.redis_client
                .mark_task_as_processing(&ctx.queue_name, &ctx.executor_id)
            => {
                match res {
                    Ok(Some(raw_data)) => {
                        let task = deserialize_raw_task_data(&raw_data)?;
                        let task_name = format!("{}:{}", &task.name, &task.id);
                        let actual_task_name = task.name.clone();

                        logger.info(format_args!(
                            "Received a task '{}' with a total of {} Bytes",
                            &task_name,
                            raw_data.len()
                        ));

                        let Some(task_data) = ctx.task_registry.get_task(Arc::new(task.name.to_string())) else {
                            logger.warn(format_args!("Task '{}' not found in registry. Skipping", &task.name));
                            if let Err(e) = ctx.redis_client
                                .remove_from_processing(&ctx.queue_name, &ctx.executor_id, &raw_data)
                                .await {
                                    logger.error(format_args!("Failed to remove the task: {}", e));
                            }
                            return Ok(());
                        };

                        let task_result = run_task(ctx.executor_id.clone(), ctx.python_dispatcher.clone(), Arc::new(task), task_data.clone()).await;

                        match task_result {
                            Ok(_) => {
                                if let Err(e) = ctx.redis_client
                                    .remove_from_processing(&ctx.queue_name, &ctx.executor_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to remove the task after successful run: {}", e));
                                }
                            }
                            Err(_) => {
                                if let Err(err) = ctx.redis_client
                                    .mark_as_failed(&ctx.queue_name, &ctx.executor_id, &raw_data)
                                    .await {
                                        logger.error(format_args!("Failed to mark the task '{}' as failed: {}", actual_task_name, err));
                                }
                            }
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        logger.error(format_args!("Worker {} redis error: {}", &ctx.executor_id, e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

async fn janitor_loop(
    mut shutdown: watch::Receiver<bool>,
    ready_check: ReadyCheck,
    queue_name: Arc<String>,
    executor_ids: Arc<Vec<Arc<String>>>,
    save_dead_tasks: Arc<bool>,
    redis_client: Arc<RedisClient>,
) -> Result<()> {
    let logger = Logger::new("JANITOR");

    ready_check.counter.fetch_add(1, Ordering::SeqCst);
    check_worker_is_ready(ready_check);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                // TODO: Add a log on shutdown to notify how many tasks are left in the queue if any.
                return Ok(())
            }

            tasks_res = async {
                redis_client.check_failed_tasks(&queue_name).await
            } => {
                match tasks_res {
                    Ok(Some(raw_data)) => {
                        let task = deserialize_raw_task_data(&raw_data)?;
                        let task_name = format!("{}:{}", &task.name, &task.id);

                        logger.info(format_args!(
                            "Received a failed task '{}': retries={}, max retries={}",
                            &task_name,
                            &task.retries,
                            &task.max_retries
                        ));

                        if task.retries == task.max_retries {
                            logger.info(format_args!(
                                "Task '{}' has reached it's max retries and will be removed from the queue",
                                &task.name
                            ));

                            if *save_dead_tasks {
                                redis_client.push_dead_task(&queue_name, raw_data).await?;
                            }

                            return Ok(())
                        }

                        redis_client.push_task(queue_name.to_string(), raw_data).await?;
                    }
                    Ok(None) => {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    },
                    Err(e) => {
                        logger.error(format_args!("Error: {}", e));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }

            hearbeat = async {
                let executor_ids = Arc::clone(&executor_ids);
                redis_client
                    .set_executors_heartbeat(executor_ids)
                    .await
            } => {
                match hearbeat {
                    Ok(()) => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        logger.error(format_args!("Hearbeat Error: {}", e));
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

async fn run_task(
    executor_id: Arc<String>,
    python_dispatcher: Arc<PythonDispatcher>,
    task: Arc<Task>,
    task_data: Arc<TaskData>,
) -> Result<()> {
    let task_name = Arc::new(format!("{}:{}", &task.name, &task.id));

    python_dispatcher
        .execute(executor_id, task_data, task_name, task)
        .await?;

    Ok(())
}

fn generate_executor_ids(num_executors: usize) -> Arc<Vec<Arc<String>>> {
    let mut ids = Vec::with_capacity(num_executors);

    for _ in 0..num_executors {
        let id: Arc<String> = Arc::from(uuid::Uuid::new_v4().to_string());
        ids.push(id);
    }

    Arc::new(ids)
}

fn check_worker_is_ready(ready_check: ReadyCheck) {
    let current = ready_check.counter.load(Ordering::SeqCst);
    if current == ready_check.concurrency.load(Ordering::SeqCst) + 1 {
        tracing::info!(
            "Worker ready ({:?} executors, 1 janitor)",
            ready_check.concurrency
        );
        tracing::info!("{}", "-".repeat(65));
    }
}

fn check_client_library_version() -> Result<()> {
    let library_version = Python::attach(|py| -> Result<String> {
        let module = py.import("fluxqueue")?;
        let version = module.getattr("__version__")?;
        Ok(version.to_string())
    })?;

    let comparison = compare_versions(MIN_CLIENT_LIB_VERSION, &library_version);
    if comparison == -1 {
        tracing::warn!(
            "Worker version '{}' is older than client library '{}'. For full functionality, update the worker to match the client version.",
            MIN_CLIENT_LIB_VERSION,
            &library_version
        );
    }

    if comparison == 1 {
        return Err(anyhow!(
            "Minimum required client library version is: {}, found: {}",
            MIN_CLIENT_LIB_VERSION,
            &library_version
        ));
    }

    Ok(())
}

fn compare_versions(v1: &str, v2: &str) -> i8 {
    let mut parts1: Vec<u32> = v1.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let mut parts2: Vec<u32> = v2.split('.').map(|p| p.parse().unwrap_or(0)).collect();

    let len = parts1.len().max(parts2.len());
    parts1.resize(len, 0);
    parts2.resize(len, 0);

    for (a, b) in parts1.iter().zip(parts2.iter()) {
        if a > b {
            return 1;
        }
        if a < b {
            return -1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::TestWriter;
    use std::sync::{Arc, Mutex};

    fn get_test_module_path(filename: &str) -> String {
        let current_dir = std::env::current_dir().unwrap();
        let test_module_path = current_dir.join("tests").join(filename);
        test_module_path.to_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_run_task_with_sync_function() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_registry = Arc::new(TaskRegistry::new(&module_path_str, "default")?);
        let dispatcher_pool = Arc::new(PythonDispatcher::new(task_registry.clone())?);

        let task = task_registry.get_task(Arc::new("task-1".to_string()));
        assert!(task.is_some());

        if let Some(task_func) = task {
            let task = Task {
                id: "test-id".to_string(),
                name: "name".to_string(),
                args: vec![144],   // ()
                kwargs: vec![128], // {}
                created_at: 0,
                retries: 0,
                max_retries: 3,
            };

            let result = run_task(
                Arc::new("test".to_string()),
                dispatcher_pool.clone(),
                Arc::new(task),
                task_func,
            )
            .await;
            assert!(!result.is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_run_task_with_async_function() -> Result<()> {
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_registry = Arc::new(TaskRegistry::new(&module_path_str, "default")?);
        let dispatcher_pool = Arc::new(PythonDispatcher::new(task_registry.clone())?);

        let task = task_registry.get_task(Arc::new("async-task".to_string()));
        assert!(task.is_some());

        if let Some(task_func) = task {
            let task = Task {
                id: "test-id".to_string(),
                name: "name".to_string(),
                args: vec![0x92, 0x01, 0x02], // (1, 2)
                kwargs: vec![128],
                created_at: 0,
                retries: 0,
                max_retries: 3,
            };

            let result = run_task(
                Arc::new("test".to_string()),
                dispatcher_pool.clone(),
                Arc::new(task),
                task_func,
            )
            .await;
            assert!(!result.is_err());
        }

        Ok(())
    }

    async fn enqueue_tasks(redis_url: &str) -> Result<()> {
        use deadpool_redis::{Config, Runtime};
        use fluxqueue_common::{
            get_redis_connection, push_task, push_task_async, serialize_task_data,
        };

        let mut redis_conn = get_redis_connection(redis_url)?;

        let sync_task = Task {
            id: "test-id".to_string(),
            name: "task-2".to_string(),
            args: vec![0x92, 0x01, 0x02], // (1, 2)
            kwargs: vec![128],
            created_at: 0,
            retries: 0,
            max_retries: 3,
        };
        let sync_task_blob = serialize_task_data(&sync_task)?;

        push_task(&mut redis_conn, "default".to_string(), sync_task_blob)?;

        let async_task = Task {
            id: "test-id-2".to_string(),
            name: "async-task".to_string(),
            args: vec![0x92, 0x01, 0x02], // (1, 2)
            kwargs: vec![128],
            created_at: 0,
            retries: 0,
            max_retries: 3,
        };
        let async_task_blob = serialize_task_data(&async_task)?;

        let redis_config = Config::from_url(redis_url);
        let redis_pool = redis_config.create_pool(Some(Runtime::Tokio1)).unwrap();
        let mut conn_manager = redis_pool.get().await?;

        push_task_async(&mut conn_manager, "default".to_string(), async_task_blob).await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_run_worker() -> Result<()> {
        use tokio::sync::watch;
        use tokio::time::{Duration, sleep};

        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = logs.clone();

        let subscriber = tracing_subscriber::fmt()
            .with_target(false)
            .with_writer(move || TestWriter {
                logs: logs_clone.clone(),
            })
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);

        let module_path_str = get_test_module_path("test_tasks_module.py");
        let redis_url = "redis://localhost:6379";

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let worker_handle = tokio::spawn(run_worker(
            shutdown_rx,
            4,
            redis_url.to_string(),
            module_path_str,
            "default".to_string(),
            false,
        ));

        sleep(Duration::from_secs(5)).await;
        enqueue_tasks(&redis_url).await?;
        sleep(Duration::from_secs(5)).await;

        let _ = shutdown_tx.send(true);

        worker_handle.await??;

        let log_lines = logs.lock().unwrap();
        let error_found = log_lines.iter().any(|line| line.contains("ERROR"));

        assert!(!error_found, "Worker emitted an error log!");

        Ok(())
    }
}
