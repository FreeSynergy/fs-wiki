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
    /// End offset of the last source range seen inside the leaf.
    pub end: usize,
    pub glue: bool,
    pub dashes: Option<String>,
    /// Thematic break: the line as written, when it was not `---`.
    pub rule: Option<String>,
    /// Text the parser did not turn into a block (a link reference definition).
    pub unparsed: bool,
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
            end: 0,
            glue: false,
            dashes: None,
            rule: None,
            unparsed: false,
            max_cells: 0,
        }
    }
    pub fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.len += s.chars().count();
    }
    /// Append `source` read as inline Markdown: its text and marks (`**b**`, `[l](u)`), or the
    /// text as written when it is more than one line of inline content.
    pub fn push_inline(&mut self, source: &str) {
        let one = crate::markdown::import_plain(source);
        let simple = match &one.blocks[..] {
            [b] => {
                b.attrs.kind == BlockKind::Paragraph
                    && b.attrs.containers.is_empty()
                    && b.attrs.block_ref.is_none()
                    && !b.text.to_string().contains(['\n', '\t', '\u{2028}'])
            }
            _ => false,
        };
        if !simple {
            self.push(&source.replace(['\n', '\t', '\u{2028}'], " "));
            return;
        }
        let b = &one.blocks[0];
        let at = self.len;
        self.push(&b.text.to_string());
        for m in b.marks.iter() {
            let mut m = m.clone();
            m.start += at;
            m.end += at;
            self.push_mark(m);
        }
    }

    /// Record a finished mark that was never "open" (code, html, footnote ref).
    pub fn push_mark(&mut self, m: Mark) {
        self.seq += 1;
        self.order.push(self.seq);
        self.marks.push(m);
    }
}
