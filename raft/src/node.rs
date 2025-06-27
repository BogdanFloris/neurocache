use crate::{Log, StateMachine};

#[derive(Default, Debug)]
pub struct RaftNode<S: StateMachine> {
    pub log: Log<S::Command>,
    pub state_machine: S,
}

impl<S: StateMachine> RaftNode<S> {
    pub fn new(state_machine: S) -> Self {
        RaftNode {
            state_machine,
            log: Log::<S::Command>::default(),
        }
    }
}
