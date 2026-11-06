//! A table block: cells separated by tab, rows by line break, in one text.
//!
//! Pattern: Adapter — cut the text at the separators and carry each cell's marks along.

use fs_components_widgets::{Align, Block as Out, Inline};
use fs_text::{Align as FsAlign, Block};

use super::inline::shaped;

/// The table of a block whose kind is `Table`.
pub(super) fn table(block: &Block, align: &[FsAlign]) -> Out {
    let text = block.text.to_string();
    let mut rows: Vec<Vec<Inline>> = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let mut row = Vec::new();
        let mut cell_start = offset;
        for cell in line.split('\t') {
            row.push(shaped(cell, block, cell_start));
            cell_start += cell.chars().count() + 1;
        }
        rows.push(row);
        offset += line.chars().count() + 1;
    }
    Out::Table {
        align: align.iter().map(|a| map_align(*a)).collect(),
        rows,
    }
}

fn map_align(a: FsAlign) -> Align {
    match a {
        FsAlign::None => Align::None,
        FsAlign::Left => Align::Left,
        FsAlign::Center => Align::Center,
        FsAlign::Right => Align::Right,
    }
}
