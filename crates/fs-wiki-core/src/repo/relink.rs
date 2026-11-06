//! Links after a move: which ones change, and how they are written afterwards.
//!
//! Pattern: Command producer — a move shifts a page or a folder (`Shift`); every link is first
//! resolved against the page it stands on (`target.rs`), then shifted, then written again in the
//! form it had (relative stays relative). A page that moves itself is handled too: its relative
//! links change although their targets stay. The changes are ordinary ops on a copy.
//! (`MovePrefix` of fs-text sees the written text only, not the page it stands on or the style.)

use std::cmp::Reverse;

use fs_text::{Block, BlockKind, Document, LinkStyle, LinkValue, MarkKind, MarkValue, Op};

use super::target::written;
use crate::path::page_of;

/// A page (`folder == false`) or a folder (`true`) that moves from `old` to `new`.
pub(super) struct Shift<'a> {
    pub old: &'a str,
    pub new: &'a str,
    pub folder: bool,
}

impl Shift<'_> {
    /// Where the path `target` goes, and whether the new place is still a folder form (so a
    /// trailing `/` may stay); `None` if it stays.
    fn map(&self, target: &str) -> Option<(String, bool)> {
        if target == self.old {
            return Some((self.new.to_string(), true));
        }
        if self.folder {
            let rest = target.strip_prefix(self.old)?.strip_prefix('/')?;
            return Some((format!("{}/{rest}", self.new), true));
        }
        // A page: a link may leave out `.md` or name the folder of an `index.md` / `README.md`.
        if self.old.strip_suffix(".md") == Some(target) {
            let stripped = self.new.strip_suffix(".md").unwrap_or(self.new);
            return Some((stripped.to_string(), false));
        }
        for name in ["/index.md", "/README.md"] {
            if self.old.strip_suffix(name).is_some_and(|b| b == target) {
                return Some(
                    match self.new.strip_suffix(name).filter(|p| !p.is_empty()) {
                        Some(parent) => (parent.to_string(), true),
                        None => (self.new.to_string(), false),
                    },
                );
            }
        }
        None
    }
}

/// The new path text of `link` (a link on the page that moves from `before` to `after`), or
/// `None` if it stays as written.
fn new_written(link: &LinkValue, before: &str, after: &str, shift: &Shift) -> Option<String> {
    let w = written(link, before)?;
    let moved = shift.map(&w.target);
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
    }
    changed.then_some(out)
}

/// Link reference definitions (`[r]: dest "title"`, stored as plain blocks) get the new target
/// too, so none keeps the old one.
fn relink_definitions(
    out: &mut Document,
    block: &Block,
    before: &str,
    after: &str,
    shift: &Shift,
) -> bool {
    if !matches!(block.attrs.kind, BlockKind::Raw | BlockKind::Html) {
        return false;
    }
    let mut edits = vec![];
    let mut offset = 0;
    for line in block.text.to_string().split_inclusive('\n') {
        if let Some((start, len)) = definition_target(line) {
            let dest: String = line.chars().skip(start).take(len).collect();
            let link = LinkValue::new(dest, LinkStyle::Inline);
            if let Some(new) = new_written(&link, before, after, shift) {
                edits.push((offset + start, offset + start + len, new));
            }
        }
        offset += line.chars().count();
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

/// (start, length) in chars of the destination of a definition line `[label]: dest ...`.
fn definition_target(line: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let lead = chars.iter().take_while(|c| **c == ' ').count();
    if lead > 3 || chars.get(lead) != Some(&'[') {
        return None;
    }
    let close =
        (lead..chars.len().saturating_sub(1)).find(|&i| chars[i] == ']' && chars[i + 1] == ':')?;
    let mut at = close + 2;
    while chars
        .get(at)
        .is_some_and(|c| c.is_whitespace() && *c != '\n')
    {
        at += 1;
    }
    let angled = chars.get(at) == Some(&'<');
    let start = at + usize::from(angled);
    let len = chars[start..]
        .iter()
        .take_while(|c| {
            if angled {
                **c != '>'
            } else {
                !c.is_whitespace()
            }
        })
        .count();
    (len > 0).then_some((start, len))
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
