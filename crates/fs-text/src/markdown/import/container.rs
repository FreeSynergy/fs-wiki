//! Containers: block quotes, lists, items, footnote definitions.
//!
//! Pattern: Builder step — containers are pushed on a stack; every leaf flushed while they
//! are open records them in its attributes (flattened composite).

use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::{BlockKind, Container};
use pulldown_cmark::{Tag, TagEnd};
use std::ops::Range;

impl Imp<'_> {
    pub(super) fn start_container(&mut self, idx: usize, tag: &Tag, range: &Range<usize>) {
        self.flush();
        let inst = self.inst();
        match tag {
            Tag::BlockQuote(_) => {
                self.containers.push(Container::Quote { inst });
                self.fresh_quote.push(true);
                self.quote_start.push((self.blocks.len(), range.clone()));
            }
            Tag::List(start) => {
                let tight = !self.loose.contains(&idx);
                self.lists.push((inst, *start, 0, tight));
            }
            Tag::Item => self.start_item(inst, range),
            Tag::FootnoteDefinition(label) => {
                self.containers.push(Container::Footnote {
                    inst,
                    label: label.to_string(),
                });
                self.foot_start.push((self.blocks.len(), range.clone()));
            }
            _ => {}
        }
    }

    fn start_item(&mut self, inst: u32, range: &Range<usize>) {
        let (list, start, n, tight) = {
            let l = self.lists.last_mut().expect("item in list");
            let r = (l.0, l.1, l.2, l.3);
            l.2 += 1;
            r
        };
        // an item's range may begin at the line break before it (nested, tab-indented items)
        let src_slice = self.src[range.start..].trim_start();
        let marker = if start.is_some() {
            src_slice
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .chars()
                .next()
                .filter(|c| matches!(c, '.' | ')'))
                .unwrap_or('.')
        } else {
            src_slice
                .chars()
                .next()
                .filter(|c| matches!(c, '-' | '*' | '+'))
                .unwrap_or('-')
        };
        self.containers.push(Container::Item {
            inst,
            list,
            number: start.map(|st| st + n),
            marker,
            tight,
            task: None,
        });
        self.item_start.push((self.blocks.len(), range.clone()));
    }

    pub(super) fn end_container(&mut self, end: TagEnd) {
        self.flush();
        match end {
            TagEnd::BlockQuote(_) => {
                if let Some(r) = self.quote_start.pop().filter(|q| q.0 == self.blocks.len()) {
                    // empty quote: the person wrote it, so keep it as an empty paragraph
                    self.empty_paragraph(&r.1);
                }
                self.containers.pop();
                self.fresh_quote.pop();
            }
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::Item | TagEnd::FootnoteDefinition => {
                let open = if matches!(end, TagEnd::Item) {
                    self.item_start.pop()
                } else {
                    self.foot_start.pop()
                };
                if let Some(r) = open.filter(|q| q.0 == self.blocks.len()) {
                    // empty list item / footnote: keep it as an empty paragraph
                    self.empty_paragraph(&r.1);
                }
                self.containers.pop();
            }
            _ => {}
        }
    }

    /// An empty block standing for a container that has no content of its own.
    fn empty_paragraph(&mut self, range: &Range<usize>) {
        let mut l = Leaf::new(BlockKind::Paragraph);
        let start = range.start
            + (self.src.len() - range.start - self.src[range.start..].trim_start().len());
        l.start = Some(start.min(range.end.saturating_sub(1)).max(range.start));
        l.end = range.end;
        self.leaf = Some(l);
        self.flush();
    }

    pub(super) fn task_marker(&mut self, done: bool) {
        if let Some(Container::Item { task, .. }) = self.containers.last_mut() {
            *task = Some(done);
        }
    }
}
