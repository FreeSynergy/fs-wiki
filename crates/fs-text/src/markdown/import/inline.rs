//! Inline events: text, code, HTML, footnote references, breaks, emphasis spans.
//!
//! Pattern: Builder step — inline content is appended to the open leaf; spans become marks
//! when their end event closes them.

use super::hints::under;
use super::state::Imp;
use crate::doc::BlockKind;
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::HARD_BREAK;
use pulldown_cmark::Tag;
use std::ops::Range;

impl Imp<'_> {
    /// Text as the model keeps it: a table cell holds no tab or line break (they separate cells
    /// and rows in the block text), and a typed U+2028 is a plain space (U+2028 is the hard
    /// break in block text).
    fn clean<'t>(&self, t: &'t str, text: bool) -> std::borrow::Cow<'t, str> {
        let in_table = matches!(
            self.leaf.as_ref().map(|l| &l.kind),
            Some(BlockKind::Table { .. })
        );
        let prose = in_table
            || matches!(
                self.leaf.as_ref().map(|l| &l.kind),
                None | Some(BlockKind::Paragraph | BlockKind::Heading { .. })
            );
        if (in_table && t.contains(['\t', '\n', '\r'])) || (text && prose && t.contains(HARD_BREAK))
        {
            t.chars()
                .map(|c| {
                    if (in_table && matches!(c, '\t' | '\n' | '\r')) || (text && c == HARD_BREAK) {
                        ' '
                    } else {
                        c
                    }
                })
                .collect::<String>()
                .into()
        } else {
            t.into()
        }
    }

    pub(super) fn text(&mut self, t: &str) {
        let t = self.clean(t, true).into_owned();
        self.leaf_mut().push(&t);
    }

    pub(super) fn code(&mut self, c: &str) {
        let c = self.clean(c, false).into_owned();
        let l = self.leaf_mut();
        let st = l.len;
        l.push(&c);
        let end = l.len;
        l.push_mark(Mark::new(MarkKind::Code, st, end, MarkValue::None));
    }

    pub(super) fn html(&mut self, h: &str) {
        let l = self.leaf_mut();
        let st = l.len;
        l.push(h);
        if l.kind != BlockKind::Html {
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
