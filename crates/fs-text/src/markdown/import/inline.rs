//! Inline events: text, code, HTML, footnote references, breaks, emphasis spans.
//!
//! Pattern: Builder step — inline content is appended to the open leaf; spans become marks
//! when their end event closes them.

use super::hints::under;
use super::state::Imp;
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::HARD_BREAK;
use pulldown_cmark::Tag;
use std::ops::Range;

impl Imp<'_> {
    pub(super) fn text(&mut self, t: &str) {
        self.leaf_mut().push(t);
    }

    pub(super) fn code(&mut self, c: &str) {
        let l = self.leaf_mut();
        let st = l.len;
        l.push(c);
        let end = l.len;
        l.push_mark(Mark::new(MarkKind::Code, st, end, MarkValue::None));
    }

    pub(super) fn html(&mut self, h: &str) {
        let l = self.leaf_mut();
        let st = l.len;
        l.push(h);
        if l.kind != crate::doc::BlockKind::Html {
            let end = l.len;
            l.push_mark(Mark::new(MarkKind::Html, st, end, MarkValue::None));
        }
    }

    pub(super) fn footnote_ref(&mut self, label: &str) {
        let l = self.leaf_mut();
        let at = l.len;
        l.push_mark(Mark::point(
            MarkKind::FootnoteRef,
            at,
            MarkValue::Name(label.to_string()),
        ));
    }

    pub(super) fn hard_break(&mut self) {
        let mut b = [0u8; 4];
        self.leaf_mut().push(HARD_BREAK.encode_utf8(&mut b));
    }

    pub(super) fn start_span(&mut self, tag: &Tag, range: &Range<usize>) {
        let (kind, value) = match tag {
            Tag::Emphasis => (MarkKind::Emphasis, under(self.src, range.start)),
            Tag::Strong => (MarkKind::Strong, under(self.src, range.start)),
            _ => (MarkKind::Strike, MarkValue::None),
        };
        self.open(kind, value);
    }
}
