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
            }
            Tag::List(start) => {
                let tight = !self.loose.contains(&idx);
                self.lists.push((inst, *start, 0, tight));
            }
            Tag::Item => self.start_item(inst, range),
            Tag::FootnoteDefinition(label) => self.containers.push(Container::Footnote {
                inst,
                label: label.to_string(),
            }),
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
        let src_slice = self.src[range.start..].trim_start_matches([' ', '\t']);
        let marker = if start.is_some() {
            src_slice
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .chars()
                .next()
                .unwrap_or('.')
        } else {
            src_slice.chars().next().unwrap_or('-')
        };
        self.containers.push(Container::Item {
            inst,
            list,
            number: start.map(|st| st + n),
            marker,
            tight,
            task: None,
        });
        self.item_start.push(self.blocks.len());
    }

    pub(super) fn end_container(&mut self, end: &TagEnd) {
        self.flush();
        match end {
            TagEnd::BlockQuote(_) => {
                self.containers.pop();
                self.fresh_quote.pop();
            }
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::Item | TagEnd::FootnoteDefinition => {
                if matches!(end, TagEnd::Item) && self.item_start.pop() == Some(self.blocks.len()) {
                    // empty list item: keep it as an empty paragraph
                    self.leaf = Some(Leaf::new(BlockKind::Paragraph));
                    self.flush();
                }
                self.containers.pop();
            }
            _ => {}
        }
    }

    pub(super) fn task_marker(&mut self, done: bool) {
        if let Some(Container::Item { task, .. }) = self.containers.last_mut() {
            *task = Some(done);
        }
    }
}
