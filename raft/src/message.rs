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
pub enum Message<S: StateMachine> {
    AppendEntries {
        prev_log_index: Index,
        prev_log_term: Term,
        entries: Vec<Entry<S::Command>>,
        leader_commit: Index,
        term: Term,
    },
    AppendEntriesResponse {
        success: bool,
        term: Term,
    },
    ClientCommand {
        command: S::Command,
    },
    ClientResponse {
        response: RaftResponse<S::Response>,
    },
}
