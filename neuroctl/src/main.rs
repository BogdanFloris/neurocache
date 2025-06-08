use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use neurod::{KvCommand, KvResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no endpoints provided")]
    NoEndpoints,
    #[error("value required for put command")]
    ValueRequired,
}

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

fn write_message(stream: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    let len: u32 = msg
        .len()
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "message too large"))?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(msg)?;
    stream.flush()?;
    Ok(())
}

fn read_message(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut msg_buf = vec![0u8; len];
    stream.read_exact(&mut msg_buf)?;
    Ok(msg_buf)
}

fn main() -> Result<(), CliError> {
    let args = Args::parse();

    if args.endpoints.is_empty() {
        return Err(CliError::NoEndpoints);
    }

    let mut stream = TcpStream::connect(&args.endpoints[0])?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let command = match args.command {
        Command::Get => KvCommand::Get { key: args.key },
        Command::Put => {
            let value = args.value.ok_or(CliError::ValueRequired)?;
            KvCommand::Put {
                key: args.key,
                value,
            }
        }
        Command::Del => KvCommand::Del { key: args.key },
    };

    let command_json = serde_json::to_vec(&command)?;
    write_message(&mut stream, &command_json)?;
    let response_bytes = read_message(&mut stream)?;
    let response: KvResponse = serde_json::from_slice(&response_bytes)?;

    match response {
        KvResponse::Ok { value } => {
            if let Some(v) = value {
                println!("{v}");
            } else {
                println!("OK");
            }
        }
        KvResponse::NotFound => {
            eprintln!("Key not found");
            std::process::exit(1);
        }
        KvResponse::NotLeader {
            leader_addr: _,
            members: _,
        } => {
            std::process::exit(1);
        }
        KvResponse::InvalidKey => {
            eprintln!("Invalid key");
            std::process::exit(1);
        }
    }

    Ok(())
}
