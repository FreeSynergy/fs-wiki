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
use std::ops::Range;

/// First and last source line (0-based) of a block.
pub(super) type Span = (usize, usize);

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
    pub item_start: Vec<(usize, Range<usize>)>,
    pub quote_start: Vec<(usize, Range<usize>)>,
    pub foot_start: Vec<(usize, Range<usize>)>,
    pub cell_end: usize,
    pub pending_sep: Option<(Block, Option<Span>)>,
    /// Start offsets of the source lines.
    pub lines: Vec<usize>,
    /// Source lines of the finished blocks (parallel to `blocks`).
    pub spans: Vec<Option<Span>>,
    /// Link reference definitions still to be placed, by source position.
    pub defs: Vec<Range<usize>>,
    pub next_def: usize,
    /// Lines that were mkdocs admonition headers before conversion.
    pub admon_lines: HashSet<usize>,
    /// Callout instances that came from admonition syntax.
    pub admon_insts: HashSet<u32>,
}

impl<'a> Imp<'a> {
    pub fn new(
        src: &'a str,
        loose: HashSet<usize>,
        defs: Vec<Range<usize>>,
        headers: &[usize],
    ) -> Self {
        let mut lines = vec![0];
        lines.extend(src.match_indices('\n').map(|(i, _)| i + 1));
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
            foot_start: Vec::new(),
            cell_end: 0,
            pending_sep: None,
            lines,
            spans: Vec::new(),
            defs,
            next_def: 0,
            admon_lines: headers.iter().copied().collect(),
            admon_insts: HashSet::new(),
        }
    }
    /// 0-based source line of a byte offset.
    pub fn line_of(&self, off: usize) -> usize {
        self.lines.partition_point(|&s| s <= off).saturating_sub(1)
    }
    /// Source lines a leaf covers (`None` when it has no source position).
    pub fn span_of(&self, leaf: &Leaf) -> Option<Span> {
        let start = leaf.start?;
        let first = self.line_of(start);
        let last = self.line_of(leaf.end.max(start + 1) - 1).max(first);
        Some((first, last))
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
        let span = self.span_of(&leaf);
        let bracket = leaf
            .start
            .is_some_and(|st| self.src[st..].trim_start().starts_with("[!"));
        let line = leaf.start.map(|st| self.line_of(st));
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
        if leaf.unparsed {
            attrs.extra.insert("unparsed".into(), "1".into());
        }
        if let Some(r) = leaf.rule.take() {
            attrs.extra.insert("rule".into(), r);
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
        if was_fresh && bracket && block.attrs.kind == BlockKind::Paragraph {
            self.try_callout(&mut block, line);
        }
        if block.attrs.containers.iter().any(
            |c| matches!(c, Container::Callout { inst, .. } if self.admon_insts.contains(inst)),
        ) {
            block.attrs.extra.insert("admonition".into(), "1".into());
        }
        self.push_block(block, span);
    }
}

/// Outer marks (opened first) come first among marks with equal ranges.
fn sorted_by_open(marks: Vec<Mark>, order: &[usize]) -> Vec<Mark> {
    let mut v: Vec<(usize, Mark)> = order.iter().copied().zip(marks).collect();
    v.sort_by_key(|(o, _)| *o);
    v.into_iter().map(|(_, m)| m).collect()
}
