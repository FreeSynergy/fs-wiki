//! Link reference definitions after a move, and in the link index.
//!
//! Pattern: Command producer — `[r]: dest "title"` is not a link mark, so `relink.rs` handles the
//! marks and this file the definitions (found by `crate::definitions`, real ones only): each
//! destination goes through the same rule as an inline link, and no definition keeps the old
//! target. A new destination with a space or an angle bracket is wrapped in `<...>`.

use fs_text::{Block, Document, LinkStyle, LinkValue, Op};

use super::relink::new_written;
use super::shift::Shift;
use crate::definitions::{destination, targets};

/// The definitions of `block` as links (for the link index).
pub(super) fn definition_links(block: &Block) -> Vec<LinkValue> {
    targets(block)
        .iter()
        .map(|t| LinkValue::new(destination(block, t), LinkStyle::Inline))
        .collect()
}

/// Give the definitions of `block` (a block of `out`) the new targets. Whether any changed.
pub(super) fn relink_definitions(
    out: &mut Document,
    block: &Block,
    before: &str,
    after: &str,
    shift: &Shift,
) -> bool {
    let mut edits = vec![];
    for t in targets(block) {
        let link = LinkValue::new(destination(block, &t), LinkStyle::Inline);
        let Some(new) = new_written(&link, before, after, shift) else {
            continue;
        };
        let wrap = !t.angled && new.contains(|c: char| c.is_whitespace() || c == '<' || c == '>');
        if wrap {
            edits.push((t.start, t.start + t.len, format!("<{new}>")));
        } else {
            edits.push((t.start, t.start + t.len, new));
        }
    }
    let changed = !edits.is_empty();
    for (start, end, insert) in edits.into_iter().rev() {
        let op = Op::Edit {
            block: block.id,
            start,
            end,
            insert,
            restore: None,
        };
        out.apply(&op).expect("valid op");
    }
    changed
}
