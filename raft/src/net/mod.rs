use std::{collections::HashMap, net::SocketAddr};

use codec::framed_stream;
use futures::sink::SinkExt;
use futures::stream::TryStreamExt;
use outbound::OutboundPool;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc,
        oneshot::{self},
    },
};
use tracing::{error, info};

use crate::{Message, NodeId, RaftError, StateMachine, INC_CHANNEL_SIZE};

pub mod codec;
pub mod outbound;

type ClientRequest<S> = (Message<S>, oneshot::Sender<Message<S>>);
type ClientReceiver<S> = mpsc::Receiver<ClientRequest<S>>;
type ClientSender<S> = mpsc::Sender<ClientRequest<S>>;

pub struct PeerNetwork<S: StateMachine + Clone + 'static> {
    node_id: NodeId,
    listen_addr: SocketAddr,
    incoming_tx: mpsc::Sender<Message<S>>,
    incoming_rx: mpsc::Receiver<Message<S>>,
    inc_client_tx: ClientSender<S>,
    inc_client_rx: ClientReceiver<S>,
    outbound_pool: OutboundPool<S>,
}

impl<S: StateMachine + Clone> PeerNetwork<S> {
    #[must_use]
    pub fn new(
        node_id: NodeId,
        listen_addr: SocketAddr,
        peer_addrs: HashMap<NodeId, SocketAddr>,
    ) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(INC_CHANNEL_SIZE);
        let (inc_client_tx, inc_client_rx) = mpsc::channel(INC_CHANNEL_SIZE);
        let outbound_pool = OutboundPool::new(peer_addrs);

        Self {
            node_id,
            listen_addr,
            incoming_tx,
            incoming_rx,
            inc_client_tx,
            inc_client_rx,
            outbound_pool,
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

    /// Sends the message to the peer using the outbound pool.
    ///
    /// # Errors
    ///
    /// Returns `RaftError` if the outbound pool send fails.
    pub async fn send_to(&self, id: NodeId, msg: Message<S>) -> Result<(), RaftError> {
        self.outbound_pool.send(id, msg).await
    }

    /// Broadcasts the message to all peers using the outbound pool.
    ///
    /// # Errors
    ///
    /// Returns `RaftError` if the outbound pool send fails.
    pub async fn broadcast(&self, msg: Message<S>) -> Result<(), RaftError> {
        for id in self.outbound_pool.peers() {
            let msg = msg.clone();
            let _ = self.outbound_pool.send(id, msg).await;
        }
        Ok(())
    }
}

async fn handle_inc_conn<S: StateMachine + Clone>(
    stream: TcpStream,
    incoming_tx: mpsc::Sender<Message<S>>,
    inc_client_tx: ClientSender<S>,
) -> Result<(), RaftError> {
    let mut framed = framed_stream::<S>(stream);
    let first_msg = framed.try_next().await?.ok_or(RaftError::Disconnected)?;
    if let Message::ClientCommand { .. } = first_msg {
        let (resp_tx, resp_rx) = oneshot::channel();
        inc_client_tx
            .send((first_msg, resp_tx))
            .await
            .map_err(|_| RaftError::Disconnected)?;
        let response = resp_rx.await?;
        framed.send(response).await?;
    } else {
        incoming_tx
            .send(first_msg)
            .await
            .map_err(|_| RaftError::Disconnected)?;
        while let Some(msg) = framed.try_next().await? {
            incoming_tx
                .send(msg)
                .await
                .map_err(|_| RaftError::Disconnected)?;
        }
    }
    Ok(())
}
