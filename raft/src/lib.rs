use serde::{de::DeserializeOwned, Serialize};

pub mod log;
pub mod message;
pub mod node;
pub mod network;
pub mod config;

pub use log::{Log, LogError, Entry, Index, Term};
pub use message::Message;
pub use node::RaftNode;
pub use network::PeerNetwork;
pub use config::Config;

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
