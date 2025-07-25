use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::{Entry, Index, NodeId, StateMachine, Term};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RaftResponse<R> {
    Ok(R),
    NotLeader {
        leader_id: NodeId,
        leader_addr: SocketAddr,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Message<S: StateMachine + Clone> {
    AppendEntries {
        prev_log_index: Index,
        prev_log_term: Term,
        leader_id: NodeId,
        entries: Vec<Entry<S::Command>>,
        leader_commit: Index,
        term: Term,
    },
    AppendEntriesResponse {
        success: bool,
        term: Term,
        follower_id: NodeId,
        match_index: Index,
    },
    ClientCommand {
        command: S::Command,
    },
    ClientResponse {
        response: RaftResponse<S::Response>,
    },
}
