use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "4")]
    workers: usize,

    #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
    redis_url: String,
}

pub async fn main() {
    let _args = Cli::parse();
}
