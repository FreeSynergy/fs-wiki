//! A document: an ordered list of blocks.
//!
//! Pattern: Composite root — lookup helpers only; edits go through [`crate::Op`].

use super::block::Block;
use super::block_kind::BlockKind;
use crate::id::{new_id, Uuid};
use serde::{Deserialize, Serialize};

/// A page / passage collection / file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Identity.
    pub id: Uuid,
    /// Blocks in order.
    pub blocks: Vec<Block>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// Empty document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: new_id(),
            blocks: Vec::new(),
        }
    }
    /// A file (Markdown, code, …) as a one-block document.
    #[must_use]
    pub fn from_file_text(text: &str) -> Self {
        Self {
            id: new_id(),
            blocks: vec![Block::new(BlockKind::Raw, text)],
        }
    }
    /// Block by id.
    #[must_use]
    pub fn block(&self, id: Uuid) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }
    /// Mutable block by id.
    pub fn block_mut(&mut self, id: Uuid) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }
    /// Position of a block.
    #[must_use]
    pub fn block_index(&self, id: Uuid) -> Option<usize> {
        self.blocks.iter().position(|b| b.id == id)
    }
    /// Text of a [`BlockKind::Raw`] one-block document.
    #[must_use]
    pub fn raw_text(&self) -> Option<String> {
        match self.blocks.as_slice() {
            [b] if b.attrs.kind == BlockKind::Raw => Some(b.text.to_string()),
            _ => None,
        }
    }
}
