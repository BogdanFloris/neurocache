use clap::{command, Parser};
use neurod::{KvStore, NeuroError};
use raft::{Config, RaftNode};
use tracing::{error, info};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    config_file: String,
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    verbosity: tracing::Level,
}

#[tokio::main]
async fn main() -> Result<(), NeuroError> {
    let args = Args::parse();
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_line_number(true)
        .with_max_level(args.verbosity)
        .with_thread_ids(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let config = Config::from_file(args.config_file.as_str())?;
    let store = KvStore::new();
    let node = RaftNode::new(&config, store);

    node.listen().await?;

    let node_handle = tokio::spawn(async move {
        if let Err(e) = node.run().await {
            error!("node run error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("shutting down...");
    node_handle.abort();

    Ok(())
}
