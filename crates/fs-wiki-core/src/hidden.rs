//! Which blocks are not content.
//!
//! Pattern: Specification (a predicate) — `[//]: # (note)` and `[//]: <> (note)` are the usual way
//! to write a comment in Markdown: a link definition nobody sees. They stay in the text and in
//! the export, but a reader does not show them and a search does not find them.

use fs_text::{Block, BlockKind};

/// Whether `block` is a Markdown comment written as a link definition.
pub fn is_hidden_block(block: &Block) -> bool {
    if !matches!(block.attrs.kind, BlockKind::Html | BlockKind::Raw) {
        return false;
    }
    let text = block.text.to_string();
    let text = text.trim_start();
    text.starts_with("[//]: #") || text.starts_with("[//]: <>")
}
