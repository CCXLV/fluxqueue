#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL not set");

    let num_workers: usize = std::env::var("WORKERS")
        .unwrap_or("4".into())
        .parse()
        .expect("msg");

    fastqueue_worker::run_worker(redis_url, num_workers).await
}
