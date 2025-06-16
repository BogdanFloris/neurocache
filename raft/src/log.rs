use std::num::TryFromIntError;

pub type Index = i64;
pub type Term = i64;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("failed to convert index to usize")]
    CastError(#[from] TryFromIntError),
    #[error("index {0} out of bounds")]
    IndexOutOfBounds(Index),
}

#[derive(Debug, PartialEq)]
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
            term: -1,
        };
        Self {
            entries: vec![sentinel],
        }
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
}

impl<C> Log<C>
where
    C: PartialEq,
{
    /// Append all supplied entries to the log, checks consistency, and implements conflict
    /// resolution.
    ///
    /// Consistency is checked by verifying the following:
    ///  * `prev_log_index` should exist in the log
    ///  * checks if the term of the entry at `prev_log_index` matches the supplied `prev_log_term`
    ///
    ///  We return `Ok(false)` if either check fails.
    ///
    /// Conflic resolution is implemented by comparing supplied `entries`
    /// with entries already in the log starting from `prev_log_index + 1`.
    /// If entries do not match, we truncate the log at that point.
    ///
    /// # Errors
    ///
    /// Returns `RaftNodeError::CastError` if `at` fails
    pub fn append_entries(
        &mut self,
        prev_log_index: Index,
        prev_log_term: Term,
        mut entries: Vec<Entry<C>>,
    ) -> Result<bool, LogError> {
        let n = i64::try_from(self.entries.len())?;
        if prev_log_index >= n {
            return Ok(false);
        }
        if self.at(prev_log_index)?.term != prev_log_term {
            return Ok(false);
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
        Ok(true)
    }
}
