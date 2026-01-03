use crate::queue::{QueueHandle, WorkerMessage, worker::run_worker};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

pub fn start_runtime(workers: usize) -> QueueHandle {
    let (tx, rx) = mpsc::channel::<WorkerMessage>(10000);

    let rt = Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
        .unwrap();

    std::thread::spawn(move || {
        rt.block_on(run_worker(rx, workers));
    });

    QueueHandle { sender: tx }
}
