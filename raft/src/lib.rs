use serde::{de::DeserializeOwned, Serialize};

pub trait StateMachine {
    type Command: Serialize + DeserializeOwned + Clone;
    type Response: Serialize + DeserializeOwned;

    fn apply(&mut self, command: Self::Command) -> Self::Response;
}

pub type Term = usize;
pub type Index = usize;

pub struct Entry<C> {
    pub command: C,
}

pub struct Log<C> {
    pub entries: Vec<Entry<C>>,
    pub committed: Index,
    pub applied: Index,
}

pub struct RaftNode<S: StateMachine> {
    log: Log<S::Command>,
    state_machine: S,
}

impl<S: StateMachine> RaftNode<S> {
    pub fn apply_committed_entries(&mut self) {
        while self.log.applied < self.log.committed {
            let entry = &self.log.entries[self.log.applied];
            self.state_machine.apply(entry.command.clone());
            self.log.applied += 1;
        }
    }
}
