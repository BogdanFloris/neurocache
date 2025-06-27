use serde::{Deserialize, Serialize};

use crate::{Entry, Index, Term};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message<C> {
    AppendEntries {
        prev_log_index: Index,
        prev_log_term: Term,
        entries: Vec<Entry<C>>,
        leader_commit: Index,
        term: Term,
    },
    AppendEntriesResponse {
        success: bool,
        term: Term,
    },
}
