//! Event dispatch: routes each parser event to the handler of its concern.
//!
//! Pattern: Visitor-style dispatch — one `match` per event kind, one handler module per
//! concern (block, container, table, front matter, link, inline).

use super::hints::{glue_before, table_dashes};
use super::state::Imp;
use crate::doc::BlockKind;
use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;

impl Imp<'_> {
    pub(super) fn event(&mut self, idx: usize, ev: &Event, range: &Range<usize>) {
        match ev {
            Event::Start(tag) => self.start(idx, tag, range),
            Event::End(end) => self.end(end),
            Event::Text(t) => self.text(t),
            Event::Code(c) => self.code(c),
            Event::InlineMath(t) | Event::DisplayMath(t) => self.text(t),
            Event::Html(h) | Event::InlineHtml(h) => self.html(h),
            Event::FootnoteReference(label) => self.footnote_ref(label),
            Event::SoftBreak => self.text("\n"),
            Event::HardBreak => self.hard_break(),
            Event::Rule => self.rule(range),
            Event::TaskListMarker(done) => self.task_marker(*done),
        }
        self.stamp_leaf(range);
    }

    /// The first event inside a leaf fixes its source start, glue and table dash hints.
    fn stamp_leaf(&mut self, range: &Range<usize>) {
        let src = self.src;
        if let Some(l) = self.leaf.as_mut() {
            if l.start.is_none() {
                l.start = Some(range.start);
                l.glue = glue_before(src, range.start);
                if let BlockKind::Table { .. } = l.kind {
                    l.dashes = table_dashes(&src[range.clone()]);
                }
            }
        }
    }

    fn start(&mut self, idx: usize, tag: &Tag, range: &Range<usize>) {
        match tag {
            Tag::Paragraph | Tag::Heading { .. } | Tag::CodeBlock(_) | Tag::HtmlBlock => {
                self.start_leaf_block(tag)
            }
            Tag::BlockQuote(_) | Tag::List(_) | Tag::Item | Tag::FootnoteDefinition(_) => {
                self.start_container(idx, tag, range)
            }
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {
                self.start_table(tag)
            }
            Tag::MetadataBlock(k) => self.start_front_matter(*k),
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough => self.start_span(tag, range),
            Tag::Link { .. } | Tag::Image { .. } => self.start_link(tag),
            _ => {}
        }
    }

    fn end(&mut self, end: &TagEnd) {
        match end {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::HtmlBlock
            | TagEnd::MetadataBlock(_)
            | TagEnd::CodeBlock
            | TagEnd::Table => self.flush(),
            TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::FootnoteDefinition => {
                self.end_container(end)
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                self.lists.pop();
            }
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Link
            | TagEnd::Image => self.close(),
            _ => {}
        }
    }
}
