//! The export interface.
//!
//! Pattern: Adapter (target interface) — our block document in, foreign text out.

use crate::doc::{Block, Document};

/// Writes the document model in a foreign format.
pub trait Exporter {
    /// Serialise a whole document.
    fn export(&self, doc: &Document) -> String;
    /// Serialise a block sequence (used to compare blocks by their file form).
    fn export_blocks(&self, blocks: &[Block]) -> String;
}
