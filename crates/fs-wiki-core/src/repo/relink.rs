//! Links after a move: which ones change, and how they are written afterwards.
//!
//! Pattern: Command producer — a move shifts a page or a folder (`Shift`); every link is first
//! resolved against the page it stands on (`target.rs`), then shifted, then written again in the
//! form it had (relative stays relative). A page that moves itself is handled too: its relative
//! links change although their targets stay. The changes are ordinary ops on a copy.
//! (`MovePrefix` of fs-text sees the written text only, not the page it stands on or the style.)

use std::cmp::Reverse;

use fs_text::{Block, Container, Document, LinkValue, MarkKind, MarkValue, Op};

use super::definitions::relink_definitions;
use super::shift::Shift;
use super::target::written;
use super::titles::title_links;
use crate::path::page_of;

/// The new path text of `link` (a link on the page that moves from `before` to `after`), or
/// `None` if it stays as written.
pub(super) fn new_written(
    link: &LinkValue,
    before: &str,
    after: &str,
    shift: &Shift,
) -> Option<String> {
    let w = written(link, before)?;
    let moved = shift.new_target(&w);
    if moved.is_none() && before == after {
        return None;
    }
    let (target, folder_form) = moved.unwrap_or_else(|| (w.target.clone(), true));
    let mut new = w.render(&target, after, w.slash() && folder_form);
    new.push_str(link.path.strip_prefix(page_of(&link.path)).unwrap_or(""));
    (new != link.path).then_some(new)
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
            if let Some(new) = new_written(link, before, after, shift) {
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
        changed |= relink_definitions(&mut out, block, before, after, shift);
        changed |= relink_titles(&mut out, block, before, after, shift);
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

/// The links in the callout titles of `block` get their new targets, as a change of the block's
/// attributes (every block of a callout carries the title).
fn relink_titles(
    out: &mut Document,
    block: &Block,
    before: &str,
    after: &str,
    shift: &Shift,
) -> bool {
    let mut attrs = block.attrs.clone();
    let mut changed = false;
    for container in &mut attrs.containers {
        let Container::Callout { title, .. } = container else {
            continue;
        };
        let mut edits = vec![];
        for l in title_links(title) {
            let link = LinkValue::new(&title[l.start..l.end], l.style);
            if let Some(new) = new_written(&link, before, after, shift) {
                edits.push((l.start, l.end, new));
            }
        }
        for (start, end, new) in edits.into_iter().rev() {
            title.replace_range(start..end, &new);
            changed = true;
        }
    }
    if changed {
        out.apply(&Op::SetAttrs {
            block: block.id,
            attrs,
        })
        .expect("valid op");
    }
    changed
}
