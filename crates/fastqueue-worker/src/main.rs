use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Start the worker")]
    Start {
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

        #[arg(
            long,
            help = "Saves dead tasks in Redis that have used all their retries yet still failed. Can be useful for debugging."
        )]
        save_dead_tasks: bool,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("fastqueue_worker=debug")
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            workers,
            redis_url,
            tasks_module_path,
            queue,
            save_dead_tasks,
        } => {
            info!("Starting FastQueue worker. Press Ctrl+C to exit gracefully.");

            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

            let worker_handle = tokio::spawn(async move {
                fastqueue_worker::run_worker(
                    shutdown_rx,
                    workers,
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
        }
    }
    Ok(())
}
