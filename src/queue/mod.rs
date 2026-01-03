use crate::queue::task::Task;
use pyo3::{Py, PyAny};
use tokio::sync::{mpsc, oneshot};

pub mod task;
pub mod worker;

pub enum WorkerMessage {
    Task(Task),
    Shutdown(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct QueueHandle {
    pub sender: mpsc::Sender<WorkerMessage>,
}

impl QueueHandle {
    pub fn enqueue(&self, func: Py<PyAny>) {
        if let Err(e) = self
            .sender
            .blocking_send(WorkerMessage::Task(Task { func }))
        {
            eprintln!("Failed to enqueue task: {:?}", e);
        }
    }

    pub fn shutdown(&self) {
        let (tx, rx) = oneshot::channel();

        // Send shutdown signal
        if self
            .sender
            .blocking_send(WorkerMessage::Shutdown(tx))
            .is_ok()
        {
            // Wait for worker to confirm shutdown
            let _ = rx.blocking_recv();
        }
    }
}
