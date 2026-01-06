#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    fastqueue_cli::cli::main().await
}
