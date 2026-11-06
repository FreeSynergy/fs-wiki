//! GFM tables: padded cells, delimiter row with the source's dash widths.
//!
//! Pattern: Template method — split the block text into cells, render cells inline, then lay
//! out header, delimiter and body rows.

use super::inline::{inline, Ctx, Variant};
use super::options::ExportOptions;
use crate::doc::{Align, Block};
use crate::mark::Mark;

fn split_rows(
    chars: &[char],
    marks: &[&Mark],
    opts: ExportOptions,
    how: Variant,
) -> Vec<Vec<String>> {
    let cx = Ctx {
        table: true,
        heading: false,
        opts,
    };
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell_start = 0;
    // marks are sorted by start: keep the ones that can touch the current cell, not all of them
    let mut active: Vec<&Mark> = Vec::new();
    let mut next = 0;
    for i in 0..=chars.len() {
        let sep = if i == chars.len() {
            Some('\n')
        } else {
            Some(chars[i]).filter(|c| *c == '\n' || *c == '\t')
        };
        if let Some(c) = sep {
            while next < marks.len() && marks[next].start <= i {
                active.push(marks[next]);
                next += 1;
            }
            active.retain(|m| m.end >= cell_start);
            row.push(inline(chars, &active, (cell_start, i), cx, how));
            cell_start = i + 1;
            if c == '\n' {
                rows.push(std::mem::take(&mut row));
            }
        }
    }
    rows
}

fn fmt_row(r: &[String], ncols: usize) -> String {
    let mut s = String::from("|");
    for c in 0..ncols {
        let cell = r.get(c).map_or("", String::as_str);
        if !cell.is_empty() {
            s.push(' ');
            s.push_str(cell);
        }
        s.push_str(" |");
    }
    s
}

fn delimiter_row(b: &Block, header: &[String], align: &[Align], ncols: usize) -> String {
    let mut d = String::from("|");
    let dashes = b.attrs.extra.get("dashes");
    let spaced = dashes.is_some_and(|d| d.starts_with('s'));
    for c in 0..ncols {
        let given: Option<usize> = dashes
            .and_then(|d| d.trim_start_matches('s').split(',').nth(c))
            .and_then(|x| x.parse().ok());
        let w = given
            .unwrap_or_else(|| header.get(c).map_or(0, |x| x.chars().count()) + 2)
            .max(3);
        let cell = match align.get(c).copied().unwrap_or(Align::None) {
            Align::None => "-".repeat(w),
            Align::Left => format!(":{}", "-".repeat(w - 1)),
            Align::Right => format!("{}:", "-".repeat(w - 1)),
            Align::Center => format!(":{}:", "-".repeat(w - 2)),
        };
        if spaced {
            d.push(' ');
            d.push_str(&cell);
            d.push(' ');
        } else {
            d.push_str(&cell);
        }
        d.push('|');
    }
    d
}

pub(super) fn render_table(
    b: &Block,
    chars: &[char],
    marks: &[&Mark],
    align: &[Align],
    opts: ExportOptions,
    how: Variant,
) -> Vec<String> {
    let rows = split_rows(chars, marks, opts, how);
    let ncols = rows.iter().map(Vec::len).fold(align.len(), usize::max);
    let mut out = Vec::new();
    if let Some(h) = rows.first() {
        out.push(fmt_row(h, ncols));
        out.push(delimiter_row(b, h, align, ncols));
    }
    out.extend(rows.iter().skip(1).map(|r| fmt_row(r, ncols)));
    out
}
