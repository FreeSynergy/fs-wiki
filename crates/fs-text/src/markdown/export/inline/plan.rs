//! Selecting, clipping and snapping the marks of an inline window.
//!
//! Pattern: Pipeline — filter by export options, clip to the window, let atomic marks win
//! over overlapping ones, drop ranges that became empty.

use crate::mark::{LinkStyle, Mark, MarkKind};
use crate::markdown::ExportOptions;

/// An exported mark clipped to the window.
pub(super) struct Em {
    pub s: usize,
    pub e: usize,
    /// Index into the marks slice.
    pub m: usize,
    pub atomic: bool,
}

/// Marks whose content is written verbatim (code, HTML, autolinks, wiki links).
fn is_atomic(m: &Mark) -> bool {
    match m.kind {
        MarkKind::Code | MarkKind::Html => true,
        MarkKind::Link | MarkKind::Image => matches!(
            m.link().map(|l| l.style),
            Some(LinkStyle::Wiki | LinkStyle::Embed | LinkStyle::Autolink)
        ),
        _ => false,
    }
}

fn exported(m: &Mark, o: ExportOptions) -> bool {
    match m.kind {
        MarkKind::Variable => o.layers,
        _ => m.kind.is_markdown_native() || (o.layers && m.kind == MarkKind::Passage),
    }
}

fn in_window(m: &Mark, lo: usize, hi: usize) -> bool {
    if m.start > hi || m.end < lo || (m.start == hi && !m.is_point()) {
        return false;
    }
    if m.is_point() {
        m.start >= lo && m.start <= hi
    } else {
        m.end > lo && m.start < hi
    }
}

pub(super) fn plan(marks: &[&Mark], lo: usize, hi: usize, opts: ExportOptions) -> Vec<Em> {
    let mut ems: Vec<Em> = marks
        .iter()
        .enumerate()
        .filter(|(_, m)| exported(m, opts) && in_window(m, lo, hi))
        .map(|(i, m)| Em {
            s: m.start.max(lo) - lo,
            e: m.end.min(hi) - lo,
            m: i,
            atomic: is_atomic(m),
        })
        .collect();
    // atomic ranges win; drop overlapping atomics, snap other boundaries
    ems.sort_by_key(|e| (e.s, std::cmp::Reverse(e.e)));
    // accepted atoms are disjoint and come in start order, so only the last one can overlap
    let mut atoms: Vec<(usize, usize)> = Vec::new();
    ems.retain(|e| {
        if !e.atomic {
            return true;
        }
        if atoms.last().is_some_and(|a| e.s < a.1) {
            return false;
        }
        atoms.push((e.s, e.e));
        true
    });
    // at most one atom contains a boundary strictly inside it
    let inside = |x: usize| {
        let i = atoms.partition_point(|a| a.0 < x);
        i.checked_sub(1).map(|i| atoms[i]).filter(|a| x < a.1)
    };
    let mut snapped = false;
    for e in ems.iter_mut().filter(|e| !e.atomic) {
        if let Some(a) = inside(e.s) {
            e.s = a.0;
            snapped = true;
        }
        if let Some(a) = inside(e.e) {
            e.e = a.1;
            snapped = true;
        }
    }
    if snapped {
        // stable: equal ranges keep their order
        ems.sort_by_key(|e| (e.s, std::cmp::Reverse(e.e)));
    }
    // non-point ranges that became empty are dropped
    ems.retain(|e| e.atomic || e.e > e.s || marks[e.m].is_point());
    ems
}

/// Lookup of the spans that start or end at a position (`ems` is sorted by start).
pub(super) struct Index {
    /// Indices into `ems`, ordered by end.
    by_end: Vec<usize>,
}

impl Index {
    pub fn new(ems: &[Em]) -> Self {
        let mut by_end: Vec<usize> = (0..ems.len()).collect();
        by_end.sort_by_key(|&i| ems[i].e);
        Self { by_end }
    }

    /// Indices of the spans that start at `x`.
    pub fn starting_at(ems: &[Em], x: usize) -> std::ops::Range<usize> {
        let lo = ems.partition_point(|e| e.s < x);
        lo..lo + ems[lo..].iter().take_while(|e| e.s == x).count()
    }

    /// Indices of the spans that start or end at `x`.
    pub fn touching<'a>(&'a self, ems: &'a [Em], x: usize) -> impl Iterator<Item = usize> + 'a {
        let lo = self.by_end.partition_point(|&i| ems[i].e < x);
        let ends = self.by_end[lo..]
            .iter()
            .copied()
            .take_while(move |&i| ems[i].e == x);
        let starts = Self::starting_at(ems, x);
        ends.chain(starts)
    }
}
