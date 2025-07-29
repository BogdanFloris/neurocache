use axum::{routing::get, Router};
use clap::{command, Parser};
use metrics_exporter_prometheus::PrometheusBuilder;
use neurod::{KvStore, NeuroError};
use raft::{metrics, Config, RaftNode};
use std::net::SocketAddr;
use tracing::{error, info};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    config_file: String,
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    verbosity: tracing::Level,
    #[arg(long, default_value = "0.0.0.0:9090")]
    metrics_addr: String,
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

    let handle = PrometheusBuilder::new()
        .set_buckets(&[
            0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
        .expect("failed to set buckets")
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    metrics::init_metrics();

    // Start metrics HTTP server
    let metrics_addr: SocketAddr = args.metrics_addr.parse().expect("Invalid metrics address");

    let app = Router::new().route("/metrics", get(move || async move { handle.render() }));

    info!("Starting metrics server on {}", metrics_addr);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(&metrics_addr)
            .await
            .expect("failed to bind metrics address");
        axum::serve(listener, app)
            .await
            .expect("metrics server failed");
    });

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
