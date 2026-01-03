use crate::queue::{QueueHandle, WorkerMessage, worker::run_worker};
use pyo3_async_runtimes::tokio::get_runtime;
use tokio::sync::mpsc;

pub fn start_runtime(max_workers: usize) -> QueueHandle {
    let (tx, rx) = mpsc::channel::<WorkerMessage>(10000);

    let rt = get_runtime();

    rt.spawn(run_worker(rx, max_workers));

    QueueHandle { sender: tx }
}
