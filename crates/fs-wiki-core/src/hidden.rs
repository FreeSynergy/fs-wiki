//! Which blocks are not displayed.
//!
//! Pattern: Specification (a predicate) — Markdown has no comment syntax, so people write them as
//! a link definition nobody sees (`[//]: # (note)`, `[note]: # (text)`, `[x]: <> (text)`) or as an
//! HTML comment (`<!-- note -->`). Such a block stays in the text and in the export and an editor
//! shows it, but a reader does not display it and a search does not find it. "Not displayed" is
//! no secret: the text is in every export.

use fs_text::{Block, BlockKind};

use crate::definitions::{destination, is_definition_block, targets};

/// Whether `block` is a comment: only definitions with the destination `#` or `<>`, or one HTML
/// comment.
pub fn is_hidden_block(block: &Block) -> bool {
    if is_definition_block(block) {
        let found = targets(block);
        return !found.is_empty()
            && found.iter().all(|t| {
                let dest = destination(block, t);
                (!t.angled && dest == "#") || (t.angled && dest.is_empty())
            });
    }
    if block.attrs.kind != BlockKind::Html {
        return false;
    }
    let text = block.text.to_string();
    let text = text.trim();
    text.starts_with("<!--") && text.ends_with("-->") && text.find("-->") == Some(text.len() - 3)
}
