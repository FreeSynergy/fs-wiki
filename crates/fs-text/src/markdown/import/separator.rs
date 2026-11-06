//! The empty comment between two lists.
//!
//! Pattern: Deferred decision (lookahead of one block) — `<!-- -->` is how two lists of the same
//! type are kept apart in Markdown. When it stands between two lists it only says "these are two
//! lists", which the model records by itself, so it is not kept as a block. Anywhere else it is
//! an ordinary HTML block.

use super::state::Imp;
use crate::doc::{Block, BlockKind, Container};

/// The text of the separator comment.
pub(super) const SEPARATOR: &str = "<!-- -->";

/// The item container at depth `d` of `b`, if `b` is inside the first `d` containers `of`.
fn item_at<'a>(b: &'a Block, of: &[Container], d: usize) -> Option<&'a Container> {
    let cs = &b.attrs.containers;
    (cs.len() > d && cs[..d] == *of)
        .then(|| &cs[d])
        .filter(|c| matches!(c, Container::Item { .. }))
}

fn is_separator(b: &Block) -> bool {
    b.attrs.kind == BlockKind::Html && b.text.to_string().trim() == SEPARATOR
}

impl Imp<'_> {
    /// Put a finished block into the document. A separator comment waits for the next block.
    pub(super) fn push_block(&mut self, block: Block) {
        if let Some(sep) = self.pending_sep.take() {
            let at = sep.attrs.containers.len();
            let after_list = self
                .blocks
                .last()
                .and_then(|p| item_at(p, &sep.attrs.containers, at));
            let before_list = item_at(&block, &sep.attrs.containers, at);
            let two_lists = matches!((after_list, before_list), (
                Some(Container::Item { list: a, .. }),
                Some(Container::Item { list: b, .. }),
            ) if a != b);
            if !two_lists {
                self.blocks.push(sep);
            }
        }
        if is_separator(&block) {
            self.pending_sep = Some(block);
        } else {
            self.blocks.push(block);
        }
    }

    /// Write out a separator that turned out not to stand between two lists.
    pub(super) fn finish(&mut self) {
        self.flush();
        if let Some(sep) = self.pending_sep.take() {
            self.blocks.push(sep);
        }
    }
}
