//! Tables: one block per table, cells joined by TAB and rows by LF.
//!
//! Pattern: Builder step — the open list stack is borrowed as a cell counter per row.

use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::{Align, BlockKind};
use pulldown_cmark::{Alignment, Tag};

fn align(a: &Alignment) -> Align {
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
                    align: al.iter().map(align).collect(),
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
