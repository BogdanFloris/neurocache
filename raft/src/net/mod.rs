use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc,
        oneshot::{self},
    },
};
use tracing::{error, info};

use crate::{Message, NodeId, RaftError, StateMachine, INC_CHANNEL_SIZE};

type ClientRequest<S> = (Message<S>, oneshot::Sender<Message<S>>);
type ClientReceiver<S> = mpsc::Receiver<ClientRequest<S>>;
type ClientSender<S> = mpsc::Sender<ClientRequest<S>>;

pub struct PeerNetwork<S: StateMachine> {
    node_id: NodeId,
    listen_addr: SocketAddr,
    incoming_tx: mpsc::Sender<Message<S>>,
    incoming_rx: mpsc::Receiver<Message<S>>,
    inc_client_tx: ClientSender<S>,
    inc_client_rx: ClientReceiver<S>,
}

impl<S: StateMachine> PeerNetwork<S> {
    #[must_use]
    pub fn new(node_id: NodeId, listen_addr: SocketAddr) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(INC_CHANNEL_SIZE);
        let (inc_client_tx, inc_client_rx) = mpsc::channel(INC_CHANNEL_SIZE);

        Self {
            node_id,
            listen_addr,
            incoming_tx,
            incoming_rx,
            inc_client_tx,
            inc_client_rx,
        }
    }

    /// Take ownership of the receivers
    pub fn take_receivers(&mut self) -> (mpsc::Receiver<Message<S>>, ClientReceiver<S>) {
        let incoming_rx = std::mem::replace(&mut self.incoming_rx, mpsc::channel(1).1);
        let client_rx = std::mem::replace(&mut self.inc_client_rx, mpsc::channel(1).1);
        (incoming_rx, client_rx)
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
        info!("peer network listening for node {}...", self.node_id);
        let listener = TcpListener::bind(self.listen_addr).await?;
        let incoming_tx = self.incoming_tx.clone();
        let inc_client_tx = self.inc_client_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("accepting connection from: {peer_addr}");
                        let incoming_tx = incoming_tx.clone();
                        let client_tx = inc_client_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_inc_conn(stream, incoming_tx, client_tx).await {
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
}

async fn handle_inc_conn<S: StateMachine>(
    mut stream: TcpStream,
    incoming_tx: mpsc::Sender<Message<S>>,
    inc_client_tx: ClientSender<S>,
) -> Result<(), RaftError> {
    let first_msg = recv_message(&mut stream).await?;
    if let Message::ClientCommand { .. } = first_msg {
        let (resp_tx, resp_rx) = oneshot::channel();
        inc_client_tx
            .send((first_msg, resp_tx))
            .await
            .map_err(|_| RaftError::Disconnected)?;
        let response = resp_rx.await?;
        send_message(&mut stream, &response).await?;
        Ok(())
    } else {
        incoming_tx
            .send(first_msg)
            .await
            .map_err(|_| RaftError::Disconnected)?;
        loop {
            let msg = recv_message(&mut stream).await?;
            incoming_tx
                .send(msg)
                .await
                .map_err(|_| RaftError::Disconnected)?;
        }
    }
}

/// Send a Message<S> to the stream using length prefixed codec.
async fn send_message<S: StateMachine>(
    stream: &mut TcpStream,
    msg: &Message<S>,
) -> Result<(), RaftError> {
    let bytes = serde_json::to_vec(&msg)?;
    let len: u32 = bytes
        .len()
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "message too large"))?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
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
