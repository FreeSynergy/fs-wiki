//! Text diff: line diff first, then char diff inside changed hunks.
//!
//! Pattern: Adapter over `similar` (Myers) — turns its diff ops into our [`TextEdit`]s.

use similar::{capture_diff_slices, Algorithm, DiffOp};

/// One diff segment with positions derived from running counters (the index fields of
/// `similar`'s Insert/Delete ops are not reliable for our purposes).
#[derive(Clone, Copy, Debug)]
pub(super) struct Seg {
    pub(super) oi: usize,
    pub(super) ol: usize,
    pub(super) ni: usize,
    pub(super) nl: usize,
    pub(super) equal: bool,
}

pub(super) fn segments<T: std::hash::Hash + Eq + Ord>(old: &[T], new: &[T]) -> Vec<Seg> {
    let (mut oi, mut ni) = (0, 0);
    let mut v = Vec::new();
    for d in capture_diff_slices(Algorithm::Myers, old, new) {
        let (ol, nl, equal) = match d {
            DiffOp::Equal { len, .. } => (len, len, true),
            DiffOp::Delete { old_len, .. } => (old_len, 0, false),
            DiffOp::Insert { new_len, .. } => (0, new_len, false),
            DiffOp::Replace {
                old_len, new_len, ..
            } => (old_len, new_len, false),
        };
        v.push(Seg {
            oi,
            ol,
            ni,
            nl,
            equal,
        });
        oi += ol;
        ni += nl;
    }
    v
}

/// A text change in char offsets of the *old* text: replace `start..end` by `insert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Start (char offset in the old text).
    pub start: usize,
    /// End (exclusive).
    pub end: usize,
    /// Replacement.
    pub insert: String,
}

/// Minimal edits turning `old` into `new`: a line diff first, then a char diff inside
/// changed hunks. Sorted ascending, non-overlapping.
#[must_use]
pub fn diff_edits(old: &str, new: &str) -> Vec<TextEdit> {
    if old == new {
        return Vec::new();
    }
    let ol: Vec<&str> = old.split_inclusive('\n').collect();
    let nl: Vec<&str> = new.split_inclusive('\n').collect();
    let offs = |lines: &[&str]| -> Vec<usize> {
        let mut v = Vec::with_capacity(lines.len() + 1);
        let mut o = 0;
        v.push(0);
        for l in lines {
            o += l.chars().count();
            v.push(o);
        }
        v
    };
    let oo = offs(&ol);
    let mut edits = Vec::new();
    for g in segments(&ol, &nl) {
        if g.equal {
            continue;
        }
        let a: String = ol[g.oi..g.oi + g.ol].concat();
        let b: String = nl[g.ni..g.ni + g.nl].concat();
        let base = oo[g.oi];
        if g.ol == 0 || g.nl == 0 {
            edits.push(TextEdit {
                start: base,
                end: base + a.chars().count(),
                insert: b,
            });
            continue;
        }
        let ac: Vec<char> = a.chars().collect();
        let bc: Vec<char> = b.chars().collect();
        for c in segments(&ac, &bc) {
            if !c.equal {
                edits.push(TextEdit {
                    start: base + c.oi,
                    end: base + c.oi + c.ol,
                    insert: bc[c.ni..c.ni + c.nl].iter().collect(),
                });
            }
        }
    }
    // coalesce touching edits: applying back to front needs non-overlapping, unambiguous ranges
    let mut merged: Vec<TextEdit> = Vec::with_capacity(edits.len());
    for e in edits {
        match merged.last_mut() {
            Some(last) if e.start <= last.end => {
                last.end = last.end.max(e.end);
                last.insert.push_str(&e.insert);
            }
            _ => merged.push(e),
        }
    }
    merged
}
