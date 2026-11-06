//! Errors of applying an operation.
//!
//! Pattern: Error enum (thiserror) — one variant per way an op can be invalid.

use crate::id::Uuid;

/// Why an [`crate::Op`] could not be applied.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApplyError {
    /// No such block.
    #[error("unknown block {0}")]
    UnknownBlock(Uuid),
    /// No such mark.
    #[error("unknown mark {0}")]
    UnknownMark(Uuid),
    /// Range outside the block text.
    #[error("range {0}..{1} outside block of length {2}")]
    OutOfRange(usize, usize, usize),
    /// Block index outside the document.
    #[error("block index {0} out of range")]
    BadIndex(usize),
}
