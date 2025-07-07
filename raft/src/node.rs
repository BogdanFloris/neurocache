use std::time::Duration;

use tracing::info;

use crate::{Config, Message, NodeId, PeerNetwork, RaftError, RaftResponse, StateMachine};

pub struct RaftNode<S: StateMachine + Clone + 'static> {
    id: NodeId,
    state_machine: S,
    peer_network: PeerNetwork<S>,
}

impl<S: StateMachine + Clone + 'static> RaftNode<S> {
    pub fn new(config: &Config, state_machine: S) -> Self {
        let peer_addrs = config.peers.iter().map(|p| (p.id, p.addr)).collect();
        let peer_network: PeerNetwork<S> = PeerNetwork::new(config.id, config.addr, peer_addrs);

        RaftNode {
            id: config.id,
            state_machine,
            peer_network,
        }
    }

    /// Starts the Raft node and listens on the supplied socket address.
    ///
    /// # Errors
    ///
    /// Returns `RaftError` if the peer networking infrastructure throws and eeror.
    pub async fn listen(&self) -> Result<(), RaftError> {
        info!("node {} listening...", self.id);
        self.peer_network.listen().await
    }

    /// Runs the loop that listens on the peer network `incoming_rx` for messages.
    ///
    /// # Errors
    /// Returns a `RaftError` if `handle_msg` fails
    pub async fn run(mut self) -> Result<(), RaftError> {
        let (mut incoming_rx, mut client_rx) = self.peer_network.take_receivers();
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    self.send_heartbeat().await?;
                }
                // Node to node message
                Some(msg) = incoming_rx.recv() => {
                    self.handle_peer_msg(msg).await?;
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

    async fn handle_peer_msg(&mut self, msg: Message<S>) -> Result<(), RaftError> {
        if let Message::AppendEntries { prev_log_index, prev_log_term, leader_id, entries, leader_commit, term } = msg {
            info!("received append entries: {prev_log_index}, {prev_log_term}, {leader_id}, {leader_commit}, {term}");
            assert!(entries.is_empty());
            let response = Message::AppendEntriesResponse { success: true, term: 0 };
            self.peer_network.send_to(leader_id, response).await?;
        }
        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<(), RaftError> {
        let heartbeat_msg = Message::AppendEntries {
            prev_log_index: 0,
            prev_log_term: 0,
            leader_id: self.id,
            entries: vec![],
            leader_commit: 0,
            term: 0,
        };

        self.peer_network.broadcast(heartbeat_msg).await
    }
}
