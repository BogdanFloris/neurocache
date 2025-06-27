use std::net::SocketAddr;

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::{error, info};

use crate::{Message, NodeId, RaftError, StateMachine, INC_CHANNEL_SIZE};

pub struct PeerNetwork<S: StateMachine> {
    node_id: NodeId,
    listen_addr: SocketAddr,
    incoming_tx: mpsc::Sender<Message<S>>,
    incoming_rx: mpsc::Receiver<Message<S>>,
}

impl<S: StateMachine> PeerNetwork<S> {
    #[must_use]
    pub fn new(node_id: NodeId, listen_addr: SocketAddr) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(INC_CHANNEL_SIZE);

        Self {
            node_id,
            listen_addr,
            incoming_tx,
            incoming_rx,
        }
    }

    /// Listens on the `listen_addr` for incoming connections.
    /// After accepting a connection, it reads the message and puts in the
    /// incoming channel using `incoming_tx`, to be processed after.
    ///
    /// # Errors
    ///
    /// Returns a `RaftError` if the function fails.
    pub async fn listen(&self) -> Result<(), RaftError>
    where
        S: 'static,
    {
        let listener = TcpListener::bind(self.listen_addr).await?;
        let incoming_tx = self.incoming_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("accepting connection from: {peer_addr}");
                        let incoming_tx = incoming_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_inc_conn(stream, incoming_tx).await {
                                error!("error handling connection: {e}");
                            }
                        });
                    }
                    Err(e) => error!("error accepting connection: {e}"),
                }
            }
        });

        Ok(())
    }

    pub async fn recv(&mut self) -> Option<Message<S>> {
        self.incoming_rx.recv().await
    }
}

async fn handle_inc_conn<S: StateMachine>(
    mut stream: TcpStream,
    incoming_tx: mpsc::Sender<Message<S>>,
) -> Result<(), RaftError> {
    loop {
        let msg = recv_message(&mut stream).await?;
        incoming_tx
            .send(msg)
            .await
            .map_err(|_| RaftError::Disconnected)?;
    }
}

/// Receive a Message<S> from the reader using a length prefixed codec.
async fn recv_message<S: StateMachine>(stream: &mut TcpStream) -> Result<Message<S>, RaftError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;

    let len = u32::from_be_bytes(len_buf) as usize;
    let mut msg_buf = vec![0u8; len];
    stream.read_exact(&mut msg_buf).await?;

    let msg = serde_json::from_slice(&msg_buf)?;
    Ok(msg)
}
