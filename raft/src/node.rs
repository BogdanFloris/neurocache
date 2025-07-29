use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use tokio::{
    sync::oneshot,
    time::{interval_at, MissedTickBehavior},
};
use tracing::{debug, info};

use crate::{
    log::{AppendOutcome, Log},
    Config, Entry, Index, Message, NodeId, PeerNetwork, RaftError, RaftResponse, StateMachine,
    Term,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

pub struct RaftNode<S: StateMachine + Clone + 'static> {
    id: NodeId,
    leader_id: NodeId,
    state: NodeState,
    current_term: Term,
    voted_for: Option<NodeId>,

    log: Log<S::Command>,
    commit_index: Index,
    last_applied: Index,

    next_index: HashMap<NodeId, Index>,
    match_index: HashMap<NodeId, Index>,
    total_nodes: usize,

    state_machine: S,
    peer_network: PeerNetwork<S>,
    peer_addrs: HashMap<NodeId, SocketAddr>,
    pending_client_requests: HashMap<Index, oneshot::Sender<Message<S>>>,
}

impl<S: StateMachine + Clone + 'static> RaftNode<S> {
    pub fn new(config: &Config, state_machine: S) -> Self {
        let peer_addrs: HashMap<NodeId, SocketAddr> =
            config.peers.iter().map(|p| (p.id, p.addr)).collect();
        let peer_network: PeerNetwork<S> =
            PeerNetwork::new(config.id, config.addr, peer_addrs.clone());

        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();
        let total_nodes = config.peers.len() + 1;

        for peer in &config.peers {
            next_index.insert(peer.id, 1);
            match_index.insert(peer.id, 0);
        }

        let state = if config.id == 1 {
            info!("node {} starting as leader", config.id);
            NodeState::Leader
        } else {
            info!("node {} starting as follower", config.id);
            NodeState::Follower
        };
        let pending_client_requests = HashMap::new();

        RaftNode {
            id: config.id,
            leader_id: 1,
            state,
            current_term: 0,
            voted_for: None,
            log: Log::new(),
            commit_index: 0,
            last_applied: 0,
            next_index,
            match_index,
            total_nodes,
            state_machine,
            peer_network,
            peer_addrs,
            pending_client_requests,
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

        let heartbeat_period = Duration::from_millis(100);
        let start = Instant::now() + heartbeat_period;
        let mut heartbeat_interval = interval_at(start.into(), heartbeat_period);
        heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    if self.state == NodeState::Leader {
                        debug!("sending heartbeat");
                        self.send_append_entries().await?;
                    }
                }
                // Node to node message
                Some(msg) = incoming_rx.recv() => {
                    self.handle_peer_msg(msg).await?;
                }
                // Client to node message
                Some((msg, resp_tx)) = client_rx.recv() => {
                    self.handle_client_msg(msg, resp_tx).await?;

                    if self.state == NodeState::Leader {
                        heartbeat_interval.reset();
                    }
                }
            }
        }
    }

    async fn handle_client_msg(
        &mut self,
        msg: Message<S>,
        resp_tx: oneshot::Sender<Message<S>>,
    ) -> Result<(), RaftError> {
        if let Message::ClientCommand { command } = msg {
            if self.state != NodeState::Leader {
                let leader_addr = self
                    .peer_addrs
                    .get(&self.leader_id)
                    .ok_or(RaftError::UnknownPeer(self.leader_id))?;
                let _ = resp_tx.send(Message::ClientResponse {
                    response: RaftResponse::NotLeader {
                        leader_id: self.leader_id,
                        leader_addr: *leader_addr,
                    },
                });
                return Ok(());
            }
            let entry = crate::log::Entry {
                term: self.current_term,
                command: command.clone(),
            };
            self.log.entries.push(entry);
            let entry_index = self.log.last_index();
            debug!("leader appended entry at index {}", entry_index);
            self.pending_client_requests.insert(entry_index, resp_tx);
            self.send_append_entries().await?;
        }
        Ok(())
    }

    async fn handle_peer_msg(&mut self, msg: Message<S>) -> Result<(), RaftError> {
        match msg {
            Message::AppendEntries {
                term,
                leader_id,
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit,
            } => {
                self.handle_append_entries(
                    term,
                    leader_id,
                    prev_log_index,
                    prev_log_term,
                    entries,
                    leader_commit,
                )
                .await?;
            }
            Message::AppendEntriesResponse {
                term,
                success,
                follower_id,
                match_index,
            } => {
                if self.state == NodeState::Leader {
                    self.handle_append_entries_response(term, success, follower_id, match_index);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_append_entries(
        &mut self,
        term: Term,
        leader_id: NodeId,
        prev_log_index: Index,
        prev_log_term: Term,
        entries: Vec<Entry<S::Command>>,
        leader_commit: Index,
    ) -> Result<(), RaftError> {
        if term > self.current_term {
            self.transition_to_follower(term, Some(leader_id));
        }

        let success = if term < self.current_term {
            false
        } else {
            match self
                .log
                .append_entries(prev_log_index, prev_log_term, entries)
            {
                Ok(AppendOutcome::Success) => {
                    if leader_commit > self.commit_index {
                        self.commit_index = leader_commit.min(self.log.last_index());
                        self.apply_committed_entries();
                    }
                    true
                }
                Ok(AppendOutcome::Conflict) | Err(_) => false,
            }
        };

        let match_idx = if success { self.log.last_index() } else { 0 };

        let response = Message::AppendEntriesResponse {
            term: self.current_term,
            success,
            follower_id: self.id,
            match_index: match_idx,
        };
        self.peer_network.send_to(leader_id, response).await?;
        Ok(())
    }

    fn handle_append_entries_response(
        &mut self,
        term: Term,
        success: bool,
        follower_id: NodeId,
        match_idx: Index,
    ) {
        if term > self.current_term {
            self.transition_to_follower(term, None);
            return;
        }

        if success {
            self.match_index.insert(follower_id, match_idx);
            self.next_index.insert(follower_id, match_idx + 1);
            debug!("follower {} matched up to index {}", follower_id, match_idx);
            self.advance_commit_index();
        } else {
            let next_idx = self.next_index.get(&follower_id).copied().unwrap_or(1);
            self.next_index
                .insert(follower_id, next_idx.saturating_sub(1).max(1));
            debug!(
                "follower {} rejected, backing off next_index to {}",
                follower_id,
                next_idx.saturating_sub(1).max(1)
            );
        }
    }

    fn apply_committed_entries(&mut self) {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            if let Some(entry) = self.log.get(self.last_applied) {
                let resp = self.state_machine.apply(entry.command.clone());

                if let Some(resp_tx) = self.pending_client_requests.remove(&self.last_applied) {
                    let _ = resp_tx.send(Message::ClientResponse {
                        response: RaftResponse::Ok(resp),
                    });
                }
            }
        }
    }

    fn advance_commit_index(&mut self) {
        if self.state != NodeState::Leader {
            return;
        }

        let last_log_index = self.log.last_index();

        for n in (self.commit_index + 1)..=last_log_index {
            let mut count = 1;

            for &match_idx in self.match_index.values() {
                if match_idx >= n {
                    count += 1;
                }
            }

            if count > self.total_nodes / 2 {
                if let Some(entry) = self.log.get(n) {
                    if entry.term == self.current_term {
                        self.commit_index = n;
                        info!("advanced commit index to {}", n);
                    }
                }
            }
        }

        self.apply_committed_entries();
    }

    fn transition_to_follower(&mut self, new_term: Term, new_leader: Option<NodeId>) {
        self.current_term = new_term;
        self.voted_for = None;
        self.state = NodeState::Follower;

        if let Some(leader) = new_leader {
            self.leader_id = leader;
        }

        // Fail pending client requests
        for (_, resp_tx) in self.pending_client_requests.drain() {
            if let Some(leader_addr) = self.peer_addrs.get(&self.leader_id) {
                let _ = resp_tx.send(Message::ClientResponse {
                    response: RaftResponse::NotLeader {
                        leader_id: self.leader_id,
                        leader_addr: *leader_addr,
                    },
                });
            }
        }
    }

    async fn send_append_entries(&self) -> Result<(), RaftError> {
        for (&peer_id, &next_idx) in &self.next_index {
            let prev_index = next_idx.saturating_sub(1);
            let prev_term = if prev_index == 0 {
                0
            } else {
                self.log.get(prev_index).map_or(0, |e| e.term)
            };

            let entries = self.log.entries_from(next_idx)?;

            let msg = Message::AppendEntries {
                term: self.current_term,
                leader_id: self.id,
                prev_log_index: prev_index,
                prev_log_term: prev_term,
                entries,
                leader_commit: self.commit_index,
            };

            self.peer_network.send_to(peer_id, msg).await?;
        }
        Ok(())
    }
}
