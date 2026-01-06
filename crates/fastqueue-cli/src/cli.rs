use clap::Parser;
use fastqueue_worker::start_worker;
use std::io::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "4")]
    workers: u32,

    #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
    redis_url: String,
}

pub async fn main() -> Result<(), Error> {
    let args = Cli::parse();

    println!("Starting FastQueue worker...");
    println!("Redis URL: {}", args.redis_url);
    println!("Workers: {}", args.workers);
    println!("Press Ctrl+C to stop\n");

    let mut worker_task = tokio::spawn(start_worker(args.redis_url, args.workers));

    tokio::select! {
        res = &mut worker_task => res?,
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down...");
            worker_task.abort();
            Ok(())
        }
    }
}
