//! Delimiter choice around runs of `*` / `_`.
//!
//! Pattern: Strategy helpers — which delimiter an emphasis span gets, given the punctuation
//! its neighbours write.

use super::delim::delim;
use super::plan::{Em, Index};
use crate::mark::{Mark, MarkKind, MarkValue};

pub(super) fn delimiter(m: &Mark, chars: &[char], e: &Em, stars: (bool, bool)) -> &'static str {
    match m.kind {
        MarkKind::Emphasis => delim(m, chars, e.s, e.e, "*", "_", stars),
        MarkKind::Strong => delim(m, chars, e.s, e.e, "**", "__", stars),
        _ => "",
    }
}

/// Does a span that writes a punctuation char sit right behind `e` (it opens where `e` ends),
/// or right in front of it (it ends where `e` starts)? Such a neighbour makes the delimiter run
/// of `e` stand next to punctuation, not next to a word. A span that keeps `_` does not count:
/// two `_` runs would fuse.
pub(super) fn punct_runs(ems: &[Em], idx: &Index, marks: &[&Mark], k: usize) -> (bool, bool) {
    let e = &ems[k];
    let punct = |o: &Em| {
        let m = marks[o.m];
        match m.kind {
            MarkKind::Emphasis | MarkKind::Strong => {
                o.e > o.s && m.value != MarkValue::Name("_".into())
            }
            MarkKind::Html => false,
            _ => o.e > o.s || m.is_point(),
        }
    };
    // behind: a span opening where `e` ends, or one enclosing `e` that closes where it does
    let behind = |j: usize| {
        let o = &ems[j];
        (o.s == e.e || (o.e == e.e && (o.s < e.s || (o.s == e.s && j < k)))) && punct(o)
    };
    // in front: a span ending where `e` starts, or one enclosing `e` that opens where it does
    let in_front = |j: usize| {
        let o = &ems[j];
        let encloses = o.e > e.e || (o.e == e.e && j < k);
        (o.e == e.s || (o.s == e.s && (encloses || marks[o.m].is_point()))) && punct(o)
    };
    (
        idx.touching(ems, e.e).any(behind),
        idx.touching(ems, e.s).any(in_front),
    )
}

/// An emphasis opened right behind a `*` run (`*a*` + `*b*`) would fuse with it; `_` keeps
/// them apart when it still parses (no word character right after the span).
pub(super) fn apart_from_star<'a>(d: &'a str, out: &str, chars: &[char], e: &Em) -> &'a str {
    let after_alnum = chars.get(e.e).copied().is_some_and(char::is_alphanumeric);
    if d == "*" && out.ends_with('*') && !out.ends_with("\\*") && !after_alnum {
        "_"
    } else {
        d
    }
}
