use raft::StateMachine;

use std::{
    str::{from_utf8, Utf8Error},
    sync::{Arc, Mutex, PoisonError},
};

use clap::{command, Parser};
use neurod::{KvCommand, KvStore};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{event, subscriber::SetGlobalDefaultError, Level};

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("tracing: {0}")]
    Tracing(#[from] SetGlobalDefaultError),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] Utf8Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown error")]
    Unknown,
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    host: String,
    #[arg(long)]
    port: u16,
}

async fn handle_conn(mut stream: TcpStream, store: Arc<Mutex<KvStore>>) -> Result<(), ServerError> {
    // Read length prefix (4 bytes big-endian)
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Read message
    let mut msg_buf = vec![0u8; len];
    stream.read_exact(&mut msg_buf).await?;
    let request = from_utf8(&msg_buf)?;
    event!(Level::INFO, "received request: {request}");

    let cmd: KvCommand = serde_json::from_str(request)?;
    let response = {
        let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
        store.apply(cmd)
    };
    let response_json = serde_json::to_vec(&response)?;

    // Write length prefix and response
    let len: u32 = response_json
        .len()
        .try_into()
        .map_err(|_| ServerError::Unknown)?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&response_json).await?;
    stream.flush().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_line_number(true)
        .with_thread_ids(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let listener = TcpListener::bind(addr).await?;
    let store = Arc::new(Mutex::new(KvStore::new()));

    loop {
        let (stream, addr) = listener.accept().await?;
        let store = Arc::clone(&store);

        tokio::spawn(async move {
            event!(Level::INFO, "connection from {addr}");
            if let Err(e) = handle_conn(stream, store).await {
                event!(Level::ERROR, "error handling connection: {e}");
            }
        });
    }
}
