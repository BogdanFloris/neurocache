use std::{collections::HashMap, net::SocketAddr, time::Duration};

use dashmap::DashMap;
use futures::SinkExt;
use tokio::{net::TcpStream, sync::mpsc};
use tracing::error;

use crate::{Message, NodeId, RaftError, StateMachine, OUT_CHANNEL_SIZE};

use super::codec::framed_stream;

pub struct OutboundPool<S: StateMachine + Clone + 'static> {
    channels: DashMap<NodeId, mpsc::Sender<Message<S>>>,
    addrs: HashMap<NodeId, SocketAddr>,
}

impl<S: StateMachine + Clone> OutboundPool<S> {
    #[must_use]
    pub fn new(addrs: HashMap<NodeId, SocketAddr>) -> Self {
        Self {
            addrs,
            channels: DashMap::new(),
        }
    }

    /// Sends a message on the peer's channel.
    /// Calls `spawn_sender` if a channel is not available for the peer.
    ///
    /// # Errors
    ///
    /// Returns `RaftError::Disconnected` if the send fails.
    pub async fn send(&self, peer: NodeId, msg: Message<S>) -> Result<(), RaftError> {
        let tx = match self.channels.get(&peer) {
            Some(ch) => ch.clone(),
            None => self.spawn_sender(peer)?,
        };
        tx.send(msg).await.map_err(|_| RaftError::Disconnected)
    }

    /// Ensures a channel for sending messages to the given peer exists.
    /// Returns its sender.
    ///
    /// # Errors
    ///
    /// Returns `RaftError` if the peer is unknown
    fn spawn_sender(&self, peer: NodeId) -> Result<mpsc::Sender<Message<S>>, RaftError> {
        let (tx, mut rx) = mpsc::channel(OUT_CHANNEL_SIZE);
        self.channels.insert(peer, tx.clone());

        let addr = self.addrs.get(&peer).ok_or(RaftError::UnknownPeer(peer))?;
        let addr = *addr;

        tokio::spawn(async move {
            loop {
                match TcpStream::connect(addr).await {
                    Ok(stream) => {
                        let mut framed = framed_stream::<S>(stream);
                        while let Some(msg) = rx.recv().await {
                            if let Err(e) = framed.send(msg).await {
                                error!(?e, %peer, "send error: reconnecting");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!(?e, %peer, "connect failed: retrying in 1s");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(tx)
    }

    pub fn peers(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.addrs.keys().copied()
    }
}
