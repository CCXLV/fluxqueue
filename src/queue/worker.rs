use crate::queue::WorkerMessage;
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;

pub async fn run_worker(mut rx: mpsc::Receiver<WorkerMessage>, workers: usize) {
    let semaphore = Arc::new(Semaphore::new(workers));
    let mut task_handles = Vec::new();

    loop {
        match rx.recv().await {
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
            Some(WorkerMessage::Shutdown(done_tx))
            | Some(WorkerMessage::AsyncShutdown(done_tx)) => {
                handle_shutdown(done_tx, &mut rx, &mut task_handles, &semaphore).await;
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

async fn handle_shutdown(
    done_tx: oneshot::Sender<()>,
    rx: &mut mpsc::Receiver<WorkerMessage>,
    task_handles: &mut Vec<JoinHandle<()>>,
    semaphore: &Arc<Semaphore>,
) {
    while let Ok(msg) = rx.try_recv() {
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

    for handle in task_handles {
        let _ = handle.await;
    }

    let _ = done_tx.send(());
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
