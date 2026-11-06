//! Grapheme-safe constructors for user edits.
//!
//! Pattern: Factory (Command) — an editor asks the document for an [`Op`]; the offsets are
//! snapped to grapheme-cluster boundaries first, so a user can neither insert into nor cut
//! half of an emoji or a combined letter. Replay, undo and external import use plain
//! [`Op::Edit`] with exact offsets instead, because they must reproduce text exactly.

use super::error::ApplyError;
use super::op::Op;
use crate::doc::Document;
use crate::id::Uuid;

impl Document {
    /// Op replacing `start..end` by `insert`. A range grows to whole grapheme clusters
    /// (so the replaced text is whole characters); an empty range (insertion) moves to the
    /// nearest boundary.
    ///
    /// # Errors
    /// [`ApplyError`] for an unknown block or a range outside the text.
    pub fn edit_op(
        &self,
        block: Uuid,
        start: usize,
        end: usize,
        insert: &str,
    ) -> Result<Op, ApplyError> {
        let b = self.block(block).ok_or(ApplyError::UnknownBlock(block))?;
        let len = b.text.len();
        if start > end || end > len {
            return Err(ApplyError::OutOfRange(start, end, len));
        }
        let (start, end) = b.text.snap_range(start, end);
        Ok(Op::Edit {
            block,
            start,
            end,
            insert: insert.to_string(),
            restore: None,
        })
    }

    /// Grapheme-safe insertion of `s` at `at`.
    ///
    /// # Errors
    /// See [`Document::edit_op`].
    pub fn insert_op(&self, block: Uuid, at: usize, s: &str) -> Result<Op, ApplyError> {
        self.edit_op(block, at, at, s)
    }

    /// Grapheme-safe deletion of `start..end` (grows to whole clusters).
    ///
    /// # Errors
    /// See [`Document::edit_op`].
    pub fn delete_op(&self, block: Uuid, start: usize, end: usize) -> Result<Op, ApplyError> {
        self.edit_op(block, start, end, "")
    }
}
