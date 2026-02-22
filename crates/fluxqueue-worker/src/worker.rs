use anyhow::Result;
use pyo3::{Py, PyAny};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::logger::{Logger, initial_logs};
use crate::redis_client::RedisClient;
use crate::task::{DispatcherPool, TaskRegistry};
use fluxqueue_common::{Task, deserialize_raw_task_data};

pub async fn run_worker(
    mut shutdown: watch::Receiver<bool>,
    concurrency: usize,
    redis_url: String,
    tasks_module_path: String,
    queue_name: String,
    save_dead_tasks: bool,
) -> Result<()> {
    let redis_client = RedisClient::new(&redis_url).await.map_err(|e| {
        tracing::error!("{}", e);
        std::process::exit(1);
    })?;
    let redis_client = Arc::new(redis_client);

    let task_registry = Arc::new(TaskRegistry::new(&tasks_module_path, &queue_name)?);
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

    let dispatcher_pool = Arc::new(DispatcherPool::new(concurrency)?);
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
            let dispatcher_pool = Arc::clone(&dispatcher_pool);

            async move {
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
                    dispatcher_pool,
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
    dispatcher_pool: Arc<DispatcherPool>,
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

                        logger.info(format_args!(
                            "Received a task '{}' with a total of {} Bytes",
                            &task_name,
                            raw_data.len()
                        ));

                        let Some(task_function) = ctx.task_registry.get(&task.name) else {
                            logger.warn(format_args!("Task '{}' not found in registry. Skipping", &task.name));
                            if let Err(e) = ctx.redis_client
                                .remove_from_processing(&ctx.queue_name, &ctx.executor_id, &raw_data)
                                .await {
                                    logger.error(format_args!("Failed to remove the task: {}", e));
                            }
                            return Ok(());
                        };

                        let task_result = run_task(ctx.executor_id.clone(), ctx.dispatcher_pool.clone(), &task, task_function).await;

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
                                        logger.error(format_args!("Failed to mark the task '{}' as failed: {}", &task.name, err));
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
    dispatcher_pool: Arc<DispatcherPool>,
    task: &Task,
    task_function: Arc<Py<PyAny>>,
) -> Result<()> {
    let task_name = Arc::new(format!("{}:{}", &task.name, &task.id));
    let raw_args = Arc::new(task.args.clone());
    let raw_kwargs = Arc::new(task.kwargs.clone());

    let dispatcher = dispatcher_pool.get();
    dispatcher
        .execute(executor_id, task_function, task_name, raw_args, raw_kwargs)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::TestWriter;
    use std::sync::{Arc, Mutex};

    // #[test]
    // fn test_path_to_module_path() -> Result<()> {
    //     let current_dir = Path::new("project");
    //     let tasks_path = Path::new("../project/tasks.py");
    //     let normalized_path = normalize_path(tasks_path);
    //     let module_path = path_to_module_path(current_dir, &normalized_path);
    //     let expected_path = Path::new("project/tasks.py");

    //     assert_eq!(normalized_path, expected_path);
    //     assert_eq!(module_path, Some("tasks".to_string()));

    //     Ok(())
    // }

    fn get_test_module_path(filename: &str) -> String {
        let current_dir = std::env::current_dir().unwrap();
        let test_module_path = current_dir.join("tests").join(filename);
        test_module_path.to_str().unwrap().to_string()
    }

    // #[test]
    // fn test_get_task_functions_valid_module() -> Result<()> {
    //     let module_path_str = get_test_module_path("test_tasks_module.py");
    //     let functions = get_registry(&module_path_str, "default")?;

    //     assert_eq!(functions.len(), 3);

    //     let task_names: Vec<String> = functions.iter().map(|(name, _)| name.clone()).collect();
    //     assert!(task_names.contains(&"task-1".to_string()));
    //     assert!(task_names.contains(&"task-2".to_string()));
    //     assert!(task_names.contains(&"async-task".to_string()));

    //     assert!(!task_names.contains(&"high-priority-task".to_string()));

    //     Ok(())
    // }

    // #[test]
    // fn test_get_task_functions_different_queue() -> Result<()> {
    //     let module_path_str = get_test_module_path("test_tasks_module.py");
    //     let functions = get_registry(&module_path_str, "high-priority")?;

    //     assert_eq!(functions.len(), 1);
    //     assert_eq!(functions[0].0, "high-priority-task");

    //     Ok(())
    // }

    // #[test]
    // fn test_get_task_functions_empty_module() -> Result<()> {
    //     let module_path_str = get_test_module_path("test_tasks_empty.py");
    //     let functions = get_registry(&module_path_str, "default")?;

    //     assert_eq!(functions.len(), 0);

    //     Ok(())
    // }

    // #[test]
    // fn test_get_task_functions_duplicate_names() {
    //     let module_path_str = get_test_module_path("test_tasks_duplicate.py");

    //     let result = get_registry(&module_path_str, "default");
    //     assert!(result.is_err());

    //     let error_msg = result.unwrap_err().to_string();
    //     assert!(error_msg.contains("duplicated") || error_msg.contains("duplicate"));
    // }

    // #[test]
    // fn test_get_task_functions_invalid_path() {
    //     let result = get_registry("nonexistent/path/to/module.py", "default");
    //     assert!(result.is_err());
    // }

    #[tokio::test]
    async fn test_run_task_with_sync_function() -> Result<()> {
        let dispatcher_pool = Arc::new(DispatcherPool::new(1)?);
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_registry = TaskRegistry::new(&module_path_str, "default")?;

        let task = task_registry.get("task-1");
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
                &task,
                task_func,
            )
            .await;
            assert!(!result.is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_run_task_with_async_function() -> Result<()> {
        let dispatcher_pool = Arc::new(DispatcherPool::new(1)?);
        let module_path_str = get_test_module_path("test_tasks_module.py");
        let task_registry = TaskRegistry::new(&module_path_str, "default")?;

        let task = task_registry.get("async-task");
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
                &task,
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
