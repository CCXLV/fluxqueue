use anyhow::Result;
use clap::Parser;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        default_value_t = 4,
        env = "FLUXQUEUE_CONCURRENCY",
        help = "Number of tasks processed in parallel."
    )]
    concurrency: usize,

    #[arg(
        short,
        long,
        default_value = "redis://127.0.0.1:6379",
        env = "FLUXQUEUE_REDIS_URL",
        help = "Redis URL to connect to."
    )]
    redis_url: String,

    #[arg(
        short,
        long,
        env = "FLUXQUEUE_TASKS_MODULE_PATH",
        help = "Module path where the task functions are exported or located."
    )]
    tasks_module_path: String,

    #[arg(
        short,
        long,
        default_value = "default",
        env = "FLUXQUEUE_QUEUE",
        help = "Name of the queue."
    )]
    queue: String,

    #[arg(
        long,
        help = "Saves dead tasks in Redis that have used all their retries yet still failed. Can be useful for debugging."
    )]
    save_dead_tasks: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("fluxqueue_worker=debug")
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let concurrency = cli.concurrency;
    let redis_url = cli.redis_url;
    let tasks_module_path = cli.tasks_module_path;
    let queue = cli.queue;
    let save_dead_tasks = cli.save_dead_tasks;

    info!("Starting fluxqueue worker. Press Ctrl+C to exit gracefully.");

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let worker_handle = tokio::spawn(async move {
        fluxqueue_worker::run_worker(
            shutdown_rx,
            concurrency,
            &redis_url,
            tasks_module_path,
            &queue,
            save_dead_tasks,
        )
        .await
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    warn!("Shutdown signal received! Starting graceful shutdown...");

    let _ = shutdown_tx.send(true);

    match worker_handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Worker exited with error: {}", e),
        Err(e) => error!("Worker task panicked: {}", e),
    }

    info!("Worker has shut down successfully.");

    Ok(())
}
