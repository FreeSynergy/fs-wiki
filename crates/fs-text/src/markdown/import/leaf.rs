//! The block under construction.
//!
//! Pattern: Builder — text and marks are accumulated here and turned into a `Block` on flush.

use crate::doc::BlockKind;
use crate::mark::{Mark, MarkKind, MarkValue};

/// An open inline mark: kind, start, value, sequence number.
pub(super) type OpenMark = (MarkKind, usize, MarkValue, usize);

pub(super) struct Leaf {
    pub kind: BlockKind,
    pub text: String,
    pub len: usize,
    pub marks: Vec<Mark>,
    pub open: Vec<OpenMark>,
    pub seq: usize,
    pub order: Vec<usize>,
    pub heading_attrs: Option<String>,
    pub start: Option<usize>,
    pub glue: bool,
    pub dashes: Option<String>,
    /// Tables: the widest row (cells).
    pub max_cells: usize,
}

impl Leaf {
    pub fn new(kind: BlockKind) -> Self {
        Self {
            kind,
            text: String::new(),
            len: 0,
            marks: Vec::new(),
            open: Vec::new(),
            seq: 0,
            order: Vec::new(),
            heading_attrs: None,
            start: None,
            glue: false,
            dashes: None,
            max_cells: 0,
        }
    }
    pub fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.len += s.chars().count();
    }
    /// Record a finished mark that was never "open" (code, html, footnote ref).
    pub fn push_mark(&mut self, m: Mark) {
        self.seq += 1;
        self.order.push(self.seq);
        self.marks.push(m);
    }
}
