use log::{Log, LogError};
use serde::{de::DeserializeOwned, Serialize};

pub mod log;

#[derive(Debug, thiserror::Error)]
pub enum RaftNodeError {
    #[error("log operation failed")]
    LogError(#[from] LogError),
}

pub trait StateMachine {
    type Command: Serialize + DeserializeOwned + Clone + PartialEq;
    type Response: Serialize + DeserializeOwned;

    fn apply(&mut self, command: Self::Command) -> Self::Response;
}

#[derive(Default, Debug)]
pub struct RaftNode<S: StateMachine> {
    pub log: Log<S::Command>,
    pub state_machine: S,
}

impl<S: StateMachine> RaftNode<S> {
    /// Applies all committed but not yet applied entries to the state machine
    ///
    /// # Errors
    ///
    /// Returns `RaftNodeError::CastError` if `at` fails
    pub fn apply_committed_entries(&mut self) -> Result<(), RaftNodeError> {
        todo!()
    }
}
