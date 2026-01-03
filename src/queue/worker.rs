use crate::python::call::call_python;
use crate::queue::WorkerMessage;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn run_worker(
    mut __rx__: tokio::sync::mpsc::Receiver<WorkerMessage>,
    max_workers: usize,
) {
    let semaphore = Arc::new(Semaphore::new(max_workers));
    let mut task_handles = Vec::new();

    loop {
        match __rx__.recv().await {
            Some(WorkerMessage::Task(task)) => {
                let sem = semaphore.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();

                    tokio::task::spawn_blocking(move || {
                        call_python(task.func);
                    })
                    .await
                    .ok();
                });

                task_handles.push(handle);
            }
            Some(WorkerMessage::Shutdown(done_tx)) => {
                // CRITICAL: Process any remaining tasks in the channel
                while let Ok(msg) = __rx__.try_recv() {
                    if let WorkerMessage::Task(task) = msg {
                        let sem = semaphore.clone();

                        let handle = tokio::spawn(async move {
                            let _permit = sem.acquire().await.unwrap();

                            tokio::task::spawn_blocking(move || {
                                call_python(task.func);
                            })
                            .await
                            .ok();
                        });

                        task_handles.push(handle);
                    }
                }

                // Wait for ALL tasks to complete
                for handle in task_handles {
                    let _ = handle.await;
                }

                // Signal that we're done
                let _ = done_tx.send(());
                break;
            }
            None => {
                // Channel closed, finish remaining tasks
                for handle in task_handles {
                    let _ = handle.await;
                }
                break;
            }
        }
    }
}
