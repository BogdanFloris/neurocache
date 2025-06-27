use clap::{command, Parser};
use neurod::NeuroError;
use raft::Config;
use tracing::info;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    config_file: String
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
    info!("{config:?}");

    Ok(())
}
