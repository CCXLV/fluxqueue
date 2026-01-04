use crate::queue::WorkerMessage;
use pyo3::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;

// Thread-local caches to avoid repeated imports and event loop creation
thread_local! {
    static INSPECT_MODULE: RefCell<Option<Py<PyModule>>> = RefCell::new(None);
    static EVENT_LOOP: RefCell<Option<Py<PyAny>>> = RefCell::new(None);
}

pub async fn run_worker(mut rx: mpsc::Receiver<WorkerMessage>, workers: usize) {
    let semaphore = Arc::new(Semaphore::new(workers));
    let mut task_handles = Vec::new();

    loop {
        match rx.recv().await {
            Some(WorkerMessage::Task(task)) => {
                let sem = semaphore.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    execute_task(task.func).await;
                });

                task_handles.push(handle);
            }
            Some(WorkerMessage::Shutdown(done_tx))
            | Some(WorkerMessage::AsyncShutdown(done_tx)) => {
                handle_shutdown(done_tx, &mut rx, &mut task_handles, &semaphore).await;
                break;
            }
            None => {
                // Channel closed, drain pending tasks
                for handle in task_handles {
                    let _ = handle.await;
                }
                break;
            }
        }
    }
}

/// Executes Python function, handling both sync and async cases
async fn execute_task(func: Py<PyAny>) {
    // Single GIL acquisition: invoke function and check return type
    let (is_async, maybe_coro) = Python::attach(|py| {
        match func.call0(py) {
            Ok(result) => {
                let bound_result = result.into_bound(py);
                let is_coro = is_coroutine_object_cached(py, &bound_result);
                if is_coro {
                    (true, Some(bound_result.unbind()))
                } else {
                    // Sync function executed inline, no further work needed
                    (false, None)
                }
            }
            Err(e) => {
                e.print(py);
                (false, None)
            }
        }
    });

    // Async functions require event loop execution in blocking context
    if is_async {
        if let Some(coro) = maybe_coro {
            tokio::task::spawn_blocking(move || {
                Python::attach(|py| {
                    let bound_coro = coro.bind(py);
                    run_coroutine_optimized(py, bound_coro.clone());
                });
            })
            .await
            .ok();
        }
    }
}

/// Runs Python coroutine using thread-local event loop cache
fn run_coroutine_optimized(py: Python<'_>, coro: Bound<PyAny>) {
    let asyncio = match py.import("asyncio") {
        Ok(m) => m,
        Err(e) => {
            e.print(py);
            return;
        }
    };

    // Lazy-init event loop per thread, reuse across coroutine executions
    let has_loop = EVENT_LOOP.with(|loop_cell| {
        let mut loop_opt = loop_cell.borrow_mut();
        if loop_opt.is_none() {
            if let Ok(new_loop) = asyncio.call_method0("new_event_loop") {
                let _ = asyncio.call_method1("set_event_loop", (&new_loop,));
                let loop_py: Py<PyAny> = new_loop.unbind();
                *loop_opt = Some(loop_py);
                true
            } else {
                false
            }
        } else {
            true
        }
    });

    if has_loop {
        EVENT_LOOP.with(|loop_cell| {
            if let Some(loop_py) = loop_cell.borrow().as_ref() {
                let bound_loop = loop_py.bind(py);
                if let Err(e) = bound_loop.call_method1("run_until_complete", (coro,)) {
                    e.print(py);
                }
            }
        });
    } else {
        // Fallback to asyncio.run() if loop init failed
        if let Err(e) = asyncio.call_method1("run", (coro,)) {
            e.print(py);
        }
    }
}

/// Checks if object is a coroutine, using cached inspect module
fn is_coroutine_object_cached(py: Python<'_>, obj: &Bound<PyAny>) -> bool {
    INSPECT_MODULE.with(|inspect_cell| {
        let mut inspect_opt = inspect_cell.borrow_mut();
        if inspect_opt.is_none() {
            if let Ok(module) = py.import("inspect") {
                *inspect_opt = Some(module.unbind());
            }
        }

        if let Some(inspect_py) = inspect_opt.as_ref() {
            let bound_inspect = inspect_py.bind(py);
            bound_inspect
                .call_method1("iscoroutine", (obj,))
                .and_then(|res| res.extract::<bool>())
                .unwrap_or(false)
        } else {
            false
        }
    })
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
                execute_task(task.func).await;
            });

            task_handles.push(handle);
        }
    }

    for handle in task_handles.drain(..) {
        let _ = handle.await;
    }

    let _ = done_tx.send(());
}
