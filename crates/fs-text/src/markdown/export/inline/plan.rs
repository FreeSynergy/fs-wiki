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
    let mut atoms: Vec<(usize, usize)> = Vec::new();
    ems.retain(|e| {
        if !e.atomic {
            return true;
        }
        if atoms.iter().any(|a| e.s < a.1 && a.0 < e.e) {
            return false;
        }
        atoms.push((e.s, e.e));
        true
    });
    for e in ems.iter_mut().filter(|e| !e.atomic) {
        for a in &atoms {
            if a.0 < e.s && e.s < a.1 {
                e.s = a.0;
            }
            if a.0 < e.e && e.e < a.1 {
                e.e = a.1;
            }
        }
    }
    // non-point ranges that became empty are dropped
    ems.retain(|e| e.atomic || e.e > e.s || marks[e.m].is_point());
    ems
}
