//! Tables: one block per table, cells joined by TAB and rows by LF.
//!
//! Pattern: Builder step — the open list stack is borrowed as a cell counter per row. Cells beyond
//! the header width are dropped by the parser; they are read back from the row's source so no
//! text is lost, and the table is widened to the widest row.

use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::{Align, BlockKind};
use pulldown_cmark::{Alignment, Tag};
use std::ops::Range;

fn align(a: Alignment) -> Align {
    match a {
        Alignment::None => Align::None,
        Alignment::Left => Align::Left,
        Alignment::Center => Align::Center,
        Alignment::Right => Align::Right,
    }
}

impl Imp<'_> {
    pub(super) fn start_table(&mut self, tag: &Tag) {
        match tag {
            Tag::Table(al) => {
                self.flush();
                self.leaf = Some(Leaf::new(BlockKind::Table {
                    align: al.iter().copied().map(align).collect(),
                }));
                self.pending_tight = true; // "no row yet"
            }
            Tag::TableHead | Tag::TableRow => {
                let first_row = self.pending_tight;
                if !first_row {
                    self.leaf_mut().push("\n");
                }
                self.pending_tight = false;
                // a counter for the cells of this row (first cell needs no tab)
                self.lists.push((u32::MAX, None, 0, true));
            }
            _ => {
                let first = self.lists.last().is_none_or(|l| l.2 == 0);
                if let Some(l) = self.lists.last_mut() {
                    l.2 += 1;
                }
                if !first {
                    self.leaf_mut().push("\t");
                }
            }
        }
    }
}

/// Cells of `rest` (the source after the last cell the parser kept): `| c | d |` gives `c`, `d`.
fn excess_cells(rest: &str) -> Vec<String> {
    let Some(rest) = rest.trim().strip_prefix('|') else {
        return Vec::new();
    };
    let (mut cells, mut cur, mut esc) = (Vec::new(), String::new(), false);
    for c in rest.chars() {
        match (esc, c) {
            (true, '|') => cur.push('|'),
            (true, _) => {
                cur.push('\\');
                cur.push(c);
            }
            (false, '\\') => {
                esc = true;
                continue;
            }
            (false, '|') => cells.push(std::mem::take(&mut cur)),
            (false, _) => cur.push(c),
        }
        esc = false;
    }
    if esc {
        cur.push('\\');
    }
    cells.push(cur);
    let mut cells: Vec<String> = cells.iter().map(|c| c.trim().to_string()).collect();
    while cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

impl Imp<'_> {
    /// End of a table row: keep the cells the parser dropped, remember the row width.
    pub(super) fn end_row(&mut self, head: bool, range: &Range<usize>) {
        let mut n = self
            .lists
            .pop()
            .map_or(0, |l| usize::try_from(l.2).unwrap_or(0));
        let from = self.cell_end.min(range.end);
        if !head {
            for cell in excess_cells(&self.src[from..range.end]) {
                let leaf = self.leaf_mut();
                leaf.push("\t");
                leaf.push_inline(&cell);
                n += 1;
            }
        }
        let leaf = self.leaf_mut();
        leaf.max_cells = leaf.max_cells.max(n);
    }
}

/// Make the table as wide as its widest row: every row is padded with empty cells (the parser
/// pads short rows itself), alignment gets the default for the new columns.
pub(super) fn widen_table(leaf: &mut Leaf) {
    let BlockKind::Table { align } = &mut leaf.kind else {
        return;
    };
    let Some(extra) = leaf.max_cells.checked_sub(align.len()).filter(|e| *e > 0) else {
        return;
    };
    align.resize(leaf.max_cells, Align::None);
    if let Some(d) = leaf.dashes.as_mut() {
        // the exporter writes `---` under an empty header cell
        d.push_str(&",3".repeat(extra));
    }
    // (char offset of a line end in the old text, tabs inserted there)
    let (mut text, mut pads, mut pos) = (String::new(), Vec::new(), 0usize);
    for (i, line) in leaf.text.split('\n').enumerate() {
        if i > 0 {
            text.push('\n');
            pos += 1;
        }
        text.push_str(line);
        pos += line.chars().count();
        let missing = leaf
            .max_cells
            .saturating_sub(line.matches('\t').count() + 1);
        text.push_str(&"\t".repeat(missing));
        if missing > 0 {
            pads.push((pos, missing));
        }
    }
    let shift = |p: usize| {
        p + pads
            .iter()
            .filter(|(at, _)| *at < p)
            .map(|(_, n)| n)
            .sum::<usize>()
    };
    for m in &mut leaf.marks {
        m.start = shift(m.start);
        m.end = shift(m.end);
    }
    leaf.len = text.chars().count();
    leaf.text = text;
}
