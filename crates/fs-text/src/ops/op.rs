//! The operation enum.
//!
//! Pattern: Command (data) — a serialisable description of one change; behaviour is in
//! [`crate::Command`].

use crate::doc::{Block, BlockAttrs};
use crate::id::Uuid;
use crate::mark::Mark;
use serde::{Deserialize, Serialize};

/// One change to a document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    /// Replace chars `start..end` of a block by `insert`. Marks follow the normal edit
    /// rules, unless `restore` is given: then the block's marks are set to that snapshot.
    Edit {
        /// Block id.
        block: Uuid,
        /// Start (char offset).
        start: usize,
        /// End (char offset, exclusive).
        end: usize,
        /// Replacement text.
        insert: String,
        /// Mark snapshot to restore (used by inverses).
        restore: Option<Vec<Mark>>,
    },
    /// Replace all marks of a block.
    SetMarks {
        /// Block id.
        block: Uuid,
        /// New marks.
        marks: Vec<Mark>,
    },
    /// Add one mark.
    AddMark {
        /// Block id.
        block: Uuid,
        /// The mark.
        mark: Mark,
    },
    /// Remove one mark.
    RemoveMark {
        /// Block id.
        block: Uuid,
        /// Mark id.
        mark: Uuid,
    },
    /// Replace block attributes.
    SetAttrs {
        /// Block id.
        block: Uuid,
        /// New attributes.
        attrs: BlockAttrs,
    },
    /// Insert a block at `index`.
    InsertBlock {
        /// Position.
        index: usize,
        /// The block.
        block: Block,
    },
    /// Remove a block.
    RemoveBlock {
        /// Block id.
        block: Uuid,
    },
}

impl Op {
    /// Insert `s` at char offset `at`.
    #[must_use]
    pub fn insert_text(block: Uuid, at: usize, s: &str) -> Op {
        Op::Edit {
            block,
            start: at,
            end: at,
            insert: s.to_string(),
            restore: None,
        }
    }
    /// Delete the char range `start..end`.
    #[must_use]
    pub fn delete_text(block: Uuid, start: usize, end: usize) -> Op {
        Op::Edit {
            block,
            start,
            end,
            insert: String::new(),
            restore: None,
        }
    }
}
