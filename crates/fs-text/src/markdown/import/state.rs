//! Importer state shared by all handlers.
//!
//! Pattern: Builder — [`Imp`] owns the finished blocks, the open container stack and the
//! current [`Leaf`]; `flush` turns the leaf into a block.

use super::blockref::extract_block_ref;
use super::leaf::Leaf;
use super::table::widen_table;
use crate::doc::{Block, BlockAttrs, BlockKind, Container};
use crate::id::new_id;
use crate::mark::{Mark, MarkKind, MarkSet, MarkValue};
use crate::text::Text;
use std::collections::HashSet;

/// One open list: instance, start number, next index, tight?
pub(super) type ListState = (u32, Option<u64>, u64, bool);

pub(super) struct Imp<'a> {
    pub src: &'a str,
    pub loose: HashSet<usize>,
    pub blocks: Vec<Block>,
    pub containers: Vec<Container>,
    pub fresh_quote: Vec<bool>,
    pub leaf: Option<Leaf>,
    pub next_inst: u32,
    pub lists: Vec<ListState>,
    pub pending_tight: bool,
    pub item_start: Vec<usize>,
    pub quote_start: Vec<usize>,
    pub cell_end: usize,
    pub pending_sep: Option<Block>,
}

impl<'a> Imp<'a> {
    pub fn new(src: &'a str, loose: HashSet<usize>) -> Self {
        Self {
            src,
            loose,
            blocks: Vec::new(),
            containers: Vec::new(),
            fresh_quote: Vec::new(),
            leaf: None,
            next_inst: 1,
            lists: Vec::new(),
            pending_tight: true,
            item_start: Vec::new(),
            quote_start: Vec::new(),
            cell_end: 0,
            pending_sep: None,
        }
    }
    pub fn inst(&mut self) -> u32 {
        let i = self.next_inst;
        self.next_inst += 1;
        i
    }
    pub fn ensure_leaf(&mut self) {
        if self.leaf.is_none() {
            self.leaf = Some(Leaf::new(BlockKind::Paragraph));
        }
    }
    /// The current leaf (created as a paragraph if none is open).
    pub fn leaf_mut(&mut self) -> &mut Leaf {
        self.leaf
            .get_or_insert_with(|| Leaf::new(BlockKind::Paragraph))
    }
    pub fn open(&mut self, k: MarkKind, v: MarkValue) {
        let l = self.leaf_mut();
        let at = l.len;
        l.seq += 1;
        let seq = l.seq;
        l.open.push((k, at, v, seq));
    }
    pub fn close(&mut self) {
        if let Some(l) = self.leaf.as_mut() {
            if let Some((k, st, v, seq)) = l.open.pop() {
                let end = l.len;
                l.marks.push(Mark::new(k, st, end, v));
                l.order.push(seq);
            }
        }
    }

    pub fn flush(&mut self) {
        let Some(mut leaf) = self.leaf.take() else {
            return;
        };
        if matches!(
            leaf.kind,
            BlockKind::Code { .. } | BlockKind::Html | BlockKind::FrontMatter
        ) && leaf.text.ends_with('\n')
        {
            leaf.text.pop();
            leaf.len -= 1;
        }
        widen_table(&mut leaf);
        let mut attrs = BlockAttrs::new(leaf.kind.clone());
        attrs.containers.clone_from(&self.containers);
        match (&leaf.kind, leaf.heading_attrs.take()) {
            (BlockKind::Heading { .. }, a) => attrs.heading_attrs = a,
            (BlockKind::FrontMatter, Some(f)) => {
                attrs.extra.insert("fence".into(), f);
            }
            _ => {}
        }
        if leaf.glue {
            attrs.extra.insert("glue".into(), "1".into());
        }
        if let Some(d) = leaf.dashes.take() {
            attrs.extra.insert("dashes".into(), d);
        }
        let marks = MarkSet::from_vec(sorted_by_open(leaf.marks, &leaf.order));
        let mut block = Block {
            id: new_id(),
            text: Text::new(&leaf.text),
            marks,
            attrs,
        };
        let was_fresh = self.fresh_quote.last().copied().unwrap_or(false);
        if let Some(f) = self.fresh_quote.last_mut() {
            *f = false;
        }
        if matches!(
            block.attrs.kind,
            BlockKind::Paragraph | BlockKind::Heading { .. }
        ) {
            extract_block_ref(&mut block);
        }
        if was_fresh && block.attrs.kind == BlockKind::Paragraph {
            self.try_callout(&mut block);
        }
        self.push_block(block);
    }
}

/// Outer marks (opened first) come first among marks with equal ranges.
fn sorted_by_open(marks: Vec<Mark>, order: &[usize]) -> Vec<Mark> {
    let mut v: Vec<(usize, Mark)> = order.iter().copied().zip(marks).collect();
    v.sort_by_key(|(o, _)| *o);
    v.into_iter().map(|(_, m)| m).collect()
}
