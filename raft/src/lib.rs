use std::io;

use serde::{de::DeserializeOwned, Serialize};

pub mod config;
pub mod log;
pub mod message;
pub mod net;
pub mod node;

pub use config::Config;
pub use log::{Entry, Log, LogError};
pub use message::{Message, RaftResponse};
pub use net::PeerNetwork;
pub use node::RaftNode;
use tokio::sync::oneshot::error::RecvError;
use tokio::task::JoinError;

pub type NodeId = u8;
pub type Index = u64;
pub type Term = u64;

pub const INC_CHANNEL_SIZE: usize = 1000;
pub const OUT_CHANNEL_SIZE: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("log operation failed")]
    Log(#[from] LogError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("rpc error: {0}")]
    Rpc(#[from] serde_json::Error),
    #[error("recv error: {0}")]
    RecvError(#[from] RecvError),
    #[error("join error: {0}")]
    JoinError(#[from] JoinError),
    #[error("disconnected")]
    Disconnected,
    #[error("unknown peer: {0}")]
    UnknownPeer(NodeId),
}

pub trait StateMachine {
    type Command: Serialize + DeserializeOwned + Clone + PartialEq + Default + Send + Sync;
    type Response: Serialize + DeserializeOwned + Clone + PartialEq + Send + Sync;

    fn apply(&mut self, command: Self::Command) -> Self::Response;
}
