use raft::StateMachine;

use std::{
    str::{from_utf8, Utf8Error},
    sync::{Arc, Mutex, PoisonError},
};

use clap::{command, Parser};
use neurod::{KvCommand, KvStore};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
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

async fn handle_conn<S>(mut stream: S, store: Arc<Mutex<KvStore>>) -> Result<(), ServerError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // Client closed connection gracefully
                event!(Level::DEBUG, "client closed connection");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
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

        let len: u32 = response_json
            .len()
            .try_into()
            .map_err(|_| ServerError::Unknown)?;
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&response_json).await?;
        stream.flush().await?;
    }
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

#[cfg(test)]
mod handle_conn {
    use super::*;
    use neurod::KvResponse;
    use tokio::io::duplex;

    async fn send<S>(c: &mut S, cmd: &KvCommand) -> KvResponse
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let msg = serde_json::to_vec(cmd).unwrap();
        let len: u32 = msg.len().try_into().unwrap();
        c.write_all(&len.to_be_bytes()).await.unwrap();
        c.write_all(&msg).await.unwrap();

        let mut len = [0u8; 4];
        c.read_exact(&mut len).await.unwrap();
        let mut msg_buf = vec![0; u32::from_be_bytes(len) as usize];
        c.read_exact(&mut msg_buf).await.unwrap();

        serde_json::from_slice(&msg_buf).unwrap()
    }

    #[tokio::test]
    async fn roundtrip_put_get() {
        let (mut client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        tokio::spawn(handle_conn(server, store));

        let put = KvCommand::Put {
            key: "k".into(),
            value: "v".into(),
        };
        assert_eq!(
            send(&mut client, &put).await,
            KvResponse::Ok { value: None }
        );
        let get = KvCommand::Get { key: "k".into() };
        assert_eq!(
            send(&mut client, &get).await,
            KvResponse::Ok {
                value: Some("v".into()),
            }
        );
    }

    #[tokio::test]
    async fn client_disconnect_graceful() {
        let (client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        let handle = tokio::spawn(handle_conn(server, store));
        drop(client);
        assert!(handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn partial_message_then_disconnect() {
        let (mut client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        let handle = tokio::spawn(handle_conn(server, store));

        client.write_all(&[0, 0]).await.unwrap();
        drop(client);

        assert!(handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn invalid_message_length() {
        let (mut client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        let handle = tokio::spawn(handle_conn(server, store));

        let len: u32 = 100;
        client.write_all(&len.to_be_bytes()).await.unwrap();
        client.write_all(b"short").await.unwrap();
        drop(client);

        assert!(handle.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn malformed_json_closes_connection() {
        let (mut client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        let handle = tokio::spawn(handle_conn(server, store));

        let bad_json = b"{invalid json}";
        let len: u32 = bad_json.len().try_into().unwrap();
        client.write_all(&len.to_be_bytes()).await.unwrap();
        client.write_all(bad_json).await.unwrap();

        let mut buf = [0u8; 1];
        assert_eq!(client.read(&mut buf).await.unwrap(), 0);

        assert!(handle.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn zero_length_message() {
        let (mut client, server) = duplex(1024);
        let store = Arc::new(Mutex::new(KvStore::new()));

        let handle = tokio::spawn(handle_conn(server, store));

        let len: u32 = 0;
        client.write_all(&len.to_be_bytes()).await.unwrap();

        drop(client);
        assert!(handle.await.unwrap().is_err());
    }
}
