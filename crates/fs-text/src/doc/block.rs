//! A block: text + marks + attributes, and the edits that keep them consistent.
//!
//! Pattern: Facade — `insert` / `splice` / `delete` move text and marks together so callers
//! cannot desynchronise them.

use super::attrs::BlockAttrs;
use super::block_kind::BlockKind;
use crate::id::{new_id, Uuid};
use crate::mark::{Mark, MarkSet};
use crate::text::Text;
use serde::{Deserialize, Serialize};

/// One block of a document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Identity.
    pub id: Uuid,
    /// Text.
    pub text: Text,
    /// Marks over the text.
    pub marks: MarkSet,
    /// Attributes.
    pub attrs: BlockAttrs,
}

impl Block {
    /// New block of `kind` holding `text`.
    #[must_use]
    pub fn new(kind: BlockKind, text: &str) -> Self {
        Self {
            id: new_id(),
            text: Text::new(text),
            marks: MarkSet::new(),
            attrs: BlockAttrs::new(kind),
        }
    }
    /// New paragraph.
    #[must_use]
    pub fn paragraph(text: &str) -> Self {
        Self::new(BlockKind::Paragraph, text)
    }

    /// Insert text; marks move per their expand rules.
    pub fn insert(&mut self, at: usize, s: &str) {
        let at = at.min(self.text.len());
        let n = s.chars().count();
        self.text.insert(at, s);
        self.marks.on_insert(at, n);
    }

    /// Replace `start..end` by `new` **keeping** marks that cover the replaced text: they
    /// are stretched or shrunk to the new text (unlike delete + insert, where a mark that
    /// covers exactly the old text would die). Marks nested inside the replaced text
    /// collapse onto the new text; points inside move to the start.
    pub fn splice(&mut self, start: usize, end: usize, new: &str) {
        let end = end.min(self.text.len());
        let start = start.min(end);
        let n = new.chars().count();
        self.text.delete(start..end);
        self.text.insert(start, new);
        let map_start = |o: usize| {
            if o <= start {
                o
            } else if o >= end {
                o - (end - start) + n
            } else {
                start
            }
        };
        let map_end = |o: usize| {
            if o <= start {
                o
            } else if o >= end {
                o - (end - start) + n
            } else {
                start + n
            }
        };
        let mut v: Vec<Mark> = self.marks.as_slice().to_vec();
        for m in &mut v {
            if m.is_point() {
                let p = if m.start > start && m.start < end {
                    start
                } else {
                    map_start(m.start)
                };
                m.start = p;
                m.end = p;
            } else {
                let (s, e) = (map_start(m.start), map_end(m.end));
                m.start = s;
                m.end = e.max(s);
            }
        }
        self.marks = MarkSet::from_vec(v);
    }

    /// Delete a char range; marks whose whole text is gone are removed and returned.
    pub fn delete(&mut self, start: usize, end: usize) -> Vec<Mark> {
        let end = end.min(self.text.len());
        let start = start.min(end);
        self.text.delete(start..end);
        self.marks.on_delete(start, end)
    }
}
