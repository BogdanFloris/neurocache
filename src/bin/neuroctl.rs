use clap::{Parser, ValueEnum};
use serde::Serialize;

#[derive(Serialize, Clone, ValueEnum, Debug)]
#[serde(rename_all = "kebab-case")]
enum Command {
    Get,
    Put,
    Del,
}

/// `neuroctl` is a command line client for `neuro`.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, env, value_delimiter = ',')]
    endpoints: Vec<String>,
    command: Command,
    key: String,
    value: Option<String>,
}

pub fn main() {
    let args = Args::parse();

    println!("Cluster endpoints: {:?}", args.endpoints);
    println!(
        "neuroctl: {:?} {:?} {:?}",
        args.command, args.key, args.value
    );
}
