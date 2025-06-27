use crate::{Config, Log, NodeId, PeerNetwork, RaftError, StateMachine};

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
}
