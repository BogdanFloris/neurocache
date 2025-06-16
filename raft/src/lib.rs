use std::num::TryFromIntError;

use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum RaftNodeError {
    #[error("")]
    CastError(#[from] TryFromIntError),
}

pub trait StateMachine {
    type Command: Serialize + DeserializeOwned + Clone;
    type Response: Serialize + DeserializeOwned;

    fn apply(&mut self, command: Self::Command) -> Self::Response;
}

pub type Term = u64;
pub type Index = u64;

#[derive(Debug)]
pub struct Entry<C> {
    pub command: C,
    pub term: Term,
}

#[derive(Default, Debug)]
pub struct Log<C> {
    pub entries: Vec<Entry<C>>,
    pub committed: Index,
    pub applied: Index,
}

#[derive(Default, Debug)]
pub struct RaftNode<S: StateMachine> {
    log: Log<S::Command>,
    state_machine: S,
}

impl<S: StateMachine> RaftNode<S> {
    /// Applies all committed but not yet applied entries to the state machine
    ///
    /// # Errors
    ///
    /// Returns `RaftNodeError::CastError` if the applied index cannot be converted
    /// to `usize` for array indexing.
    pub fn apply_committed_entries(&mut self) -> Result<(), RaftNodeError> {
        while self.log.applied < self.log.committed {
            let entry = &self.log.entries[usize::try_from(self.log.applied)?];
            self.state_machine.apply(entry.command.clone());
            self.log.applied += 1;
        }

        Ok(())
    }
}
