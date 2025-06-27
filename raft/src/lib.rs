use serde::{de::DeserializeOwned, Serialize};

pub mod config;
pub mod log;
pub mod message;
pub mod network;
pub mod node;

pub use config::Config;
pub use log::{Entry, Index, Log, LogError, Term};
pub use message::Message;
pub use network::PeerNetwork;
pub use node::RaftNode;

#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("log operation failed")]
    LogError(#[from] LogError),
}

pub trait StateMachine {
    type Command: Serialize + DeserializeOwned + Clone + PartialEq + Default;
    type Response: Serialize + DeserializeOwned;

    fn apply(&mut self, command: Self::Command) -> Self::Response;
}
