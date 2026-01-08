use clap::Parser;
use std::env;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "4", help = "Number of workers to run.")]
    workers: Option<usize>,

    #[arg(
        short,
        long,
        default_value = "redis://127.0.0.1:6379",
        help = "Redis URL to connect to."
    )]
    redis_url: Option<String>,

    #[arg(
        short,
        long,
        help = "Module path where the task functions are exported or located,."
    )]
    tasks_module_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_env_filter("fastqueue_worker=debug")
        .with_target(false)
        .init();

    let args = Cli::parse();

    let workers = args
        .workers
        .or_else(|| env::var("WORKERS").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(4);
    let redis_url = args
        .redis_url
        .or_else(|| env::var("REDIS_URL").ok().and_then(|v| v.parse().ok()))
        .unwrap_or("redis://127.0.0.1:6379".to_string());

    let tasks_module_path = args
        .tasks_module_path
        .or_else(|| {
            env::var("TASKS_MODULE_PATH")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .expect("TASKS_MODULE_PATH must be passed.");

    info!("Starting FastQueue worker. Press Ctrl+C to exit gracefully.");

    tokio::select! {
        res = fastqueue_worker::run_worker(workers, redis_url, tasks_module_path) => {
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
