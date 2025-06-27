use clap::{command, Parser};
use neurod::{KvStore, NeuroError};
use raft::{Config, RaftNode};
use tracing::info;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    config_file: String,
}

#[tokio::main]
async fn main() -> Result<(), NeuroError> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_line_number(true)
        .with_thread_ids(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    let config = Config::from_file(args.config_file.as_str())?;
    let store = KvStore::new();
    let node = RaftNode::new(&config, store);

    node.listen().await?;
    tokio::signal::ctrl_c().await?;
    info!("shutting down...");

    Ok(())
}
