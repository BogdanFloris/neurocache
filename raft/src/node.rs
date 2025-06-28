use crate::{Config, Log, Message, NodeId, PeerNetwork, RaftError, RaftResponse, StateMachine};

pub struct RaftNode<S: StateMachine> {
    id: NodeId,
    node_count: usize,
    log: Log<S::Command>,
    state_machine: S,
    peers: Vec<NodeId>,
    peer_network: PeerNetwork<S>,
}

impl<S: StateMachine + 'static> RaftNode<S> {
    pub fn new(config: &Config, state_machine: S) -> Self {
        let peers: Vec<NodeId> = config.peers.iter().map(|p| p.id).collect();
        let peer_network: PeerNetwork<S> = PeerNetwork::new(config.id, config.addr);

        RaftNode {
            id: config.id,
            node_count: peers.len() + 1,
            state_machine,
            log: Log::<S::Command>::default(),
            peers,
            peer_network,
        }
    }

    /// Starts the Raft node and listens on the supplied socket address.
    ///
    /// # Errors
    ///
    /// Returns `RaftError` if the peer networking infrastructure throws and eeror.
    pub async fn listen(&self) -> Result<(), RaftError> {
        self.peer_network.listen().await
    }

    /// Runs the loop that listens on the peer network `incoming_rx` for messages.
    ///
    /// # Errors
    /// Returns a `RaftError` if `handle_msg` fails
    pub async fn run(mut self) -> Result<(), RaftError> {
        let (mut incoming_rx, mut client_rx) = self.peer_network.take_receivers();

        loop {
            tokio::select! {
                // Node to node message
                Some(..) = incoming_rx.recv() => {
                    todo!();
                }
                // Client to node message
                Some((msg, resp_tx)) = client_rx.recv() => {
                    if let Some(resp) = self.handle_client_msg(msg) {
                        let _ = resp_tx.send(Message::ClientResponse {
                            response: RaftResponse::Ok(resp),
                        });
                    }
                }
            }
        }
    }

    fn handle_client_msg(&mut self, msg: Message<S>) -> Option<S::Response> {
        match msg {
            Message::ClientCommand { command } => Some(self.state_machine.apply(command)),
            _ => None,
        }
    }
}
