use clap::Parser;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        default_value_t = 4,
        env = "FASTQUEUE_WORKERS",
        help = "Number of workers to run."
    )]
    workers: usize,

    #[arg(
        short,
        long,
        default_value = "redis://127.0.0.1:6379",
        env = "FASTQUEUE_REDIS_URL",
        help = "Redis URL to connect to."
    )]
    redis_url: String,

    #[arg(
        short,
        long,
        env = "FASTQUEUE_TASKS_MODULE_PATH",
        help = "Module path where the task functions are exported or located,."
    )]
    tasks_module_path: String,

    #[arg(
        short,
        long,
        default_value = "default",
        env = "FASTQUEUE_QUEUE",
        help = "Name of the queue if you plan to run multiple worker processes."
    )]
    queue: String,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_env_filter("fastqueue_worker=debug")
        .with_target(false)
        .init();

    let args = Cli::parse();

    info!("Starting FastQueue worker. Press Ctrl+C to exit gracefully.");

    tokio::select! {
        res = fastqueue_worker::run_worker(
            args.workers,
            args.redis_url,
            args.tasks_module_path,
            args.queue
        ) => {
            if let Err(e) = res {
                error!("Worker stopped with error: {}", e);
            }
        }

        _ = tokio::signal::ctrl_c() => {
            warn!("Shutdown signal received! Starting graceful shutdown...");
        }
    }

    info!("Worker has shut down successfully.");
    Ok(())
}
