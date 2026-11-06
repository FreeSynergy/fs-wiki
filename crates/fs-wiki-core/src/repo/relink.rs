//! Links after a move: which ones change, and how they are written afterwards.
//!
//! Pattern: Command producer — a move shifts a page or a folder (`Shift`); every link is first
//! resolved against the page it stands on (`target.rs`), then shifted, then written again in the
//! form it had (relative stays relative). A page that moves itself is handled too: its relative
//! links change although their targets stay. The changes are ordinary ops on a copy.
//! (`MovePrefix` of fs-text sees the written text only, not the page it stands on or the style.)

use std::cmp::Reverse;

use fs_text::{Block, Document, MarkKind, MarkValue, Op};

use super::target::written;
use crate::path::page_of;

/// A page (`folder == false`) or a folder (`true`) that moves from `old` to `new`.
pub(super) struct Shift<'a> {
    pub old: &'a str,
    pub new: &'a str,
    pub folder: bool,
}

impl Shift<'_> {
    /// Where the path `target` goes; `None` if it stays.
    fn map(&self, target: &str) -> Option<String> {
        if target == self.old {
            return Some(self.new.to_string());
        }
        let rest = target.strip_prefix(self.old)?.strip_prefix('/')?;
        self.folder.then(|| format!("{}/{rest}", self.new))
    }
}

/// The document of the page that moves from `before` to `after`, with its links as they must be
/// written afterwards; `None` if no link changes. `shift` is the move of the others (or the page
/// itself).
pub(super) fn relinked(
    doc: &Document,
    before: &str,
    after: &str,
    shift: &Shift,
) -> Option<Document> {
    let mut out = doc.clone();
    let mut changed = false;
    for block in &doc.blocks {
        let mut todo = vec![];
        for mark in &block.marks {
            if !matches!(mark.kind, MarkKind::Link | MarkKind::Image) {
                continue;
            }
            let Some(link) = mark.link() else { continue };
            let Some(w) = written(link, before) else {
                continue;
            };
            let target = shift.map(&w.target);
            if target.is_none() && before == after {
                continue;
            }
            let mut new = w.render(target.as_deref().unwrap_or(&w.target), after);
            if let Some(fragment) = link.path.strip_prefix(page_of(&link.path)) {
                new.push_str(fragment);
            }
            if new != link.path {
                todo.push((mark.id, mark.start, new));
            }
        }
        todo.sort_by_key(|t| Reverse(t.1));
        for (mark, _, new) in todo {
            let current = out.block(block.id).expect("block exists");
            let op = rewrite_op(current, mark, new);
            out.apply(&op).expect("valid op");
            changed = true;
        }
    }
    changed.then_some(out)
}

/// The op that gives link mark `mark` of `block` the path `new` (and the shown text, when the
/// text is the path itself).
fn rewrite_op(block: &Block, mark: fs_text::id::Uuid, new: String) -> Op {
    let m = block.marks.get(mark).expect("mark exists");
    let (start, end) = (m.start, m.end);
    let old = m.link().expect("link").path.clone();
    let shown = block.text.slice(start..end) == old;
    let mut tmp = block.clone();
    if shown {
        tmp.splice(start, end, &new);
    }
    let mut marks = tmp.marks.as_slice().to_vec();
    if let Some(MarkValue::Link(l)) = marks
        .iter_mut()
        .find(|x| x.id == mark)
        .map(|x| &mut x.value)
    {
        l.path.clone_from(&new);
    }
    if shown {
        Op::Edit {
            block: block.id,
            start,
            end,
            insert: new,
            restore: Some(marks),
        }
    } else {
        Op::SetMarks {
            block: block.id,
            marks,
        }
    }
}
