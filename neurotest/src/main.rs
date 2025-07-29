use clap::{Parser, Subcommand};
use colored::Colorize;
use std::error::Error;

mod bench;
mod cluster;
mod config;
mod monitor;

use crate::cluster::ClusterManager;
use crate::config::Config;

#[derive(Parser)]
#[command(author, version, about = "NeuroCache test cluster management tool")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a test cluster
    Start {
        /// Number of nodes to start
        #[arg(short, long, default_value = "3")]
        nodes: usize,
        /// Run in single-node mode (no consensus)
        #[arg(long)]
        single: bool,
    },
    /// Stop the running cluster
    Stop,
    /// Show cluster status
    Status,
    /// Tail node logs
    Logs {
        /// Node ID to show logs for (all nodes if not specified)
        node_id: Option<usize>,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
    /// Run connectivity tests
    Test,
    /// Run load benchmarks
    Bench {
        /// Benchmark duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
        /// Number of concurrent threads
        #[arg(short, long, default_value = "8")]
        threads: usize,
        /// Read/write ratio (0.0 = all writes, 1.0 = all reads)
        #[arg(short, long, default_value = "0.5")]
        read_ratio: f64,
    },
    /// Manage monitoring stack
    Monitor {
        #[command(subcommand)]
        cmd: MonitorCmd,
    },
    /// Clean up all resources
    Clean {
        /// Also clean logs
        #[arg(long)]
        logs: bool,
    },
}

#[derive(Subcommand)]
enum MonitorCmd {
    /// Start monitoring stack (Prometheus + Grafana)
    Start,
    /// Stop monitoring stack
    Stop,
    /// Show monitoring status
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(false)
        .without_time()
        .init();

    // Load configuration
    let config = Config::load()?;
    let manager = ClusterManager::new(config);

    match cli.command {
        Commands::Start { nodes, single } => {
            println!("{}", "Starting NeuroCache test cluster...".green().bold());
            manager.start(nodes, single).await?;
        }
        Commands::Stop => {
            println!("{}", "Stopping NeuroCache cluster...".yellow().bold());
            manager.stop().await?;
        }
        Commands::Status => {
            manager.status().await?;
        }
        Commands::Logs { node_id, follow } => {
            manager.show_logs(node_id, follow).await?;
        }
        Commands::Test => {
            println!("{}", "Running connectivity tests...".yellow().bold());
            manager.test().await?;
        }
        Commands::Bench {
            duration,
            threads,
            read_ratio,
        } => {
            println!("{}", "Running load benchmark...".yellow().bold());
            bench::run_benchmark(duration, threads, read_ratio).await?;
        }
        Commands::Monitor { cmd } => match cmd {
            MonitorCmd::Start => {
                println!("{}", "Starting monitoring stack...".yellow().bold());
                monitor::start().await?;
            }
            MonitorCmd::Stop => {
                println!("{}", "Stopping monitoring stack...".yellow().bold());
                monitor::stop().await?;
            }
            MonitorCmd::Status => {
                monitor::status().await?;
            }
        },
        Commands::Clean { logs } => {
            println!("{}", "Cleaning up resources...".yellow().bold());
            manager.clean(logs).await?;
        }
    }

    Ok(())
}
