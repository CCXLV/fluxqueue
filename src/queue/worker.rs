use crate::queue::WorkerMessage;
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn run_worker(mut rx: tokio::sync::mpsc::Receiver<WorkerMessage>, max_workers: usize) {
    let semaphore = Arc::new(Semaphore::new(max_workers));
    let mut task_handles = Vec::new();

    loop {
        match rx.recv().await {
            Some(WorkerMessage::Task(task)) => {
                let sem = semaphore.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();

                    tokio::task::spawn(async move {
                        call_python(task.func);
                    })
                    .await
                    .ok();
                });

                task_handles.push(handle);
            }
            Some(WorkerMessage::Shutdown(done_tx)) => {
                // CRITICAL: Process any remaining tasks in the channel
                while let Ok(msg) = rx.try_recv() {
                    if let WorkerMessage::Task(task) = msg {
                        let sem = semaphore.clone();

                        let handle = tokio::spawn(async move {
                            let _permit = sem.acquire().await.unwrap();

                            tokio::task::spawn(async move {
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

fn call_python(func: Py<PyAny>) {
    Python::attach(|py| {
        if let Err(e) = func.call0(py) {
            e.print(py);
        }
    });
}

// fn is_coroutine(py: Python<'_>, func: &Bound<PyAny>) -> bool {
//     let inspect = py.import("inspect").ok();
//     if let Some(inspect_mod) = inspect {
//         inspect_mod
//             .call_method1("iscoroutine", (func,))
//             .and_then(|res| res.extract::<bool>())
//             .unwrap_or(false)
//     } else {
//         false
//     }
// }
