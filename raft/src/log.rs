use std::num::TryFromIntError;

use serde::{Deserialize, Serialize};

use crate::{Index, Term};

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("failed to convert index to usize")]
    CastError(#[from] TryFromIntError),
    #[error("index {0} out of bounds")]
    IndexOutOfBounds(Index),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    Success,  // entries appended
    Conflict, // prev_log_index/term mismatch
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Entry<C> {
    pub command: C,
    pub term: Term,
}

#[derive(Debug)]
pub struct Log<C> {
    pub entries: Vec<Entry<C>>,
}

impl<C: Default> Default for Log<C> {
    fn default() -> Self {
        let sentinel = Entry {
            command: C::default(),
            term: 0,
        };
        Self {
            entries: vec![sentinel],
        }
    }
}

impl<C: Default> Log<C> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<C> Log<C> {
    /// Returns the element at `idx` in the Log
    ///
    /// # Errors
    ///
    /// Returns `RaftNodeError::CastError` if the applied index cannot
    /// cannot be converted to `usize` for array indexing.
    pub fn at(&self, idx: Index) -> Result<&Entry<C>, LogError> {
        let index = usize::try_from(idx)?;
        self.entries
            .get(index)
            .ok_or(LogError::IndexOutOfBounds(idx))
    }

    #[must_use]
    pub fn get(&self, idx: Index) -> Option<&Entry<C>> {
        match usize::try_from(idx) {
            Ok(i) => self.entries.get(i),
            Err(_) => None,
        }
    }

    #[must_use]
    pub fn last_index(&self) -> Index {
        (self.entries.len() - 1) as Index
    }

    #[must_use]
    pub fn last_term(&self) -> Term {
        self.entries.last().map_or(0, |e| e.term)
    }
}

impl<C: Clone> Log<C> {
    /// Retrives the entries from the log starting from `start`
    ///
    /// # Errors
    ///
    /// Returns `LogError::CastError` if casting fails.
    pub fn entries_from(&self, start: Index) -> Result<Vec<Entry<C>>, LogError> {
        let start_idx = usize::try_from(start)?;
        if start_idx >= self.entries.len() {
            Ok(Vec::new())
        } else {
            Ok(self.entries[start_idx..].to_vec())
        }
    }
}

impl<C> Log<C>
where
    C: PartialEq,
{
    /// Appends entries to the log following the Raft consistency protocol.
    ///
    /// Verified log consistency at `prev_log_index` with `prev_log_term`, then:
    ///  - Removes any conflicting entries starting from the first mismatch
    ///  - Appends new entries not already present in the log
    ///
    /// # Errors
    ///
    /// Returns `RaftNodeError::CastError` if `at` or `try_from` integer casting fails
    pub fn append_entries(
        &mut self,
        prev_log_index: Index,
        prev_log_term: Term,
        mut entries: Vec<Entry<C>>,
    ) -> Result<AppendOutcome, LogError> {
        if prev_log_index >= self.entries.len() as Index {
            return Ok(AppendOutcome::Conflict);
        }
        if self.at(prev_log_index)?.term != prev_log_term {
            return Ok(AppendOutcome::Conflict);
        }
        let mut i = 0;
        let mut j = usize::try_from(prev_log_index)? + 1;
        while i < entries.len() && j < self.entries.len() {
            if entries[i] != self.entries[j] {
                self.entries.truncate(j);
                break;
            }
            i += 1;
            j += 1;
        }

        self.entries.append(&mut entries.split_off(i));
        Ok(AppendOutcome::Success)
    }
}

#[cfg(test)]
mod log_tests {
    use super::*;

    #[test]
    fn test_default_log_has_sentinel() {
        let log = Log::<u32>::default();
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].term, 0);
        assert_eq!(log.entries[0].command, 0);
    }

    #[test]
    fn test_at_valid_index() {
        let log = Log::<u32>::default();
        let entry = log.at(0).unwrap();
        assert_eq!(entry.term, 0);
        assert_eq!(entry.command, 0);
    }

    #[test]
    fn test_at_index_out_of_bounds() {
        let log = Log::<u32>::default();
        let result = log.at(1);
        assert!(matches!(result, Err(LogError::IndexOutOfBounds(1))));
    }

    #[test]
    fn test_append_entries_to_empty_log() {
        let mut log = Log::<u32>::default();
        let entries = vec![
            Entry {
                command: 1,
                term: 1,
            },
            Entry {
                command: 2,
                term: 1,
            },
        ];

        let result = log.append_entries(0, 0, entries).unwrap();
        assert_eq!(result, AppendOutcome::Success);
        assert_eq!(log.entries.len(), 3);
        assert_eq!(log.at(1).unwrap().command, 1);
        assert_eq!(log.at(2).unwrap().command, 2);
    }

    #[test]
    fn test_append_entries_idempotent() {
        let mut log = Log::<u32>::default();
        let entries = vec![
            Entry {
                command: 1,
                term: 1,
            },
            Entry {
                command: 2,
                term: 1,
            },
        ];

        log.append_entries(0, 0, entries.clone()).unwrap();
        let result = log.append_entries(0, 0, entries).unwrap();
        assert_eq!(result, AppendOutcome::Success);
        assert_eq!(log.entries.len(), 3); // Should not duplicate
    }

    #[test]
    fn test_append_entries_conflict_wrong_prev_index() {
        let mut log = Log::<u32>::default();
        let result = log.append_entries(5, 1, vec![]).unwrap();
        assert_eq!(result, AppendOutcome::Conflict);
    }

    #[test]
    fn test_append_entries_conflict_wrong_prev_term() {
        let mut log = Log::<u32>::default();
        log.entries.push(Entry {
            command: 1,
            term: 1,
        });
        let result = log.append_entries(1, 2, vec![]).unwrap();
        assert_eq!(result, AppendOutcome::Conflict);
    }

    #[test]
    fn test_append_truncates_conflicting_entries() {
        let mut log = Log::<u32>::default();
        log.entries.push(Entry {
            command: 1,
            term: 1,
        });
        log.entries.push(Entry {
            command: 2,
            term: 1,
        });
        log.entries.push(Entry {
            command: 3,
            term: 1,
        });
        let new_entries = [
            Entry {
                command: 2,
                term: 3,
            },
            Entry {
                command: 3,
                term: 3,
            },
        ];
        let result = log.append_entries(1, 1, new_entries.to_vec()).unwrap();
        assert_eq!(result, AppendOutcome::Success);
        assert_eq!(log.entries.len(), 4);
        assert_eq!(log.at(2).unwrap().command, 2);
        assert_eq!(log.at(2).unwrap().term, 3);
    }
}
