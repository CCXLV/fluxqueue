use crate::queue::{QueueHandle, WorkerMessage, worker::run_worker};
use pyo3::prelude::*;
use std::sync::Once;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

static INIT: Once = Once::new();

pub fn start_runtime(py: Python<'_>, max_workers: usize) -> QueueHandle {
    let (tx, rx) = mpsc::channel::<WorkerMessage>(10000);

    py.detach(|| {
        std::thread::spawn(move || {
            INIT.call_once(|| {
                Python::initialize();
            });

            let rt = Runtime::new().unwrap();
            rt.block_on(run_worker(rx, max_workers));
        });
    });

    QueueHandle { sender: tx }
}
