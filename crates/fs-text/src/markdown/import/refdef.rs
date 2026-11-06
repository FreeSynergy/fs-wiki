//! Link reference definitions (`[label]: /url "title"`, also the `[//]: # (comment)` idiom).
//!
//! Pattern: Recogniser — the parser swallows a definition without an event; its source span is
//! turned into an HTML-kind block marked `unparsed`, so text a person typed is never dropped.
//! The block is placed where the definition stands (inside the containers open at that point).

use super::hints::glue_before;
use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::BlockKind;
use pulldown_cmark::Event;
use std::ops::Range;

impl Imp<'_> {
    /// Place the definitions that stand before the event `ev` (called before each event).
    pub(super) fn definitions_before(&mut self, ev: &Event, range: &Range<usize>) {
        while let Some(d) = self.defs.get(self.next_def).cloned() {
            let before = match ev {
                Event::End(_) => range.end > d.start,
                _ => range.start >= d.start,
            };
            if !before || self.leaf.is_some() {
                break;
            }
            self.next_def += 1;
            self.definition_block(&d);
        }
    }

    /// Definitions after the last event.
    pub(super) fn trailing_definitions(&mut self) {
        while let Some(d) = self.defs.get(self.next_def).cloned() {
            self.next_def += 1;
            self.definition_block(&d);
        }
    }

    fn definition_block(&mut self, d: &Range<usize>) {
        let text = self.src[d.clone()].trim_end().to_string();
        let mut l = Leaf::new(BlockKind::Html);
        l.start = Some(d.start);
        l.end = d.start + text.len().max(1);
        l.glue = glue_before(self.src, d.start);
        l.unparsed = true;
        l.push(&text);
        self.leaf = Some(l);
        self.flush();
    }
}
