//! The inline sweep: walks the text once, opening and closing marks.
//!
//! Pattern: Strategy client — what a mark writes is decided by its [`MarkWriter`]; this code
//! only keeps the open-mark stack consistent (re-opening marks that were closed early by an
//! overlapping neighbour). Spans are looked up by position (sorted by start, `Index` by end),
//! so the sweep is linear in text plus marks.

use super::atomic::{atomic_str, point_str};
use super::ctx::Ctx;
use super::delim::delim;
use super::escape::{escape_into, Esc, Facts};
use super::plan::{plan, Em, Index};
use super::variant::Variant;
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::{md_syntax, MarkWriter};

fn delimiter(m: &Mark, chars: &[char], e: &Em, stars: (bool, bool)) -> &'static str {
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
fn punct_runs(ems: &[Em], idx: &Index, marks: &[&Mark], k: usize) -> (bool, bool) {
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

fn open_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).open(d)
}

fn close_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).close(m, d)
}

/// An emphasis opened right behind a `*` run (`*a*` + `*b*`) would fuse with it; `_` keeps
/// them apart when it still parses (no word character right after the span).
fn apart_from_star<'a>(d: &'a str, out: &str, chars: &[char], e: &Em) -> &'a str {
    let after_alnum = chars.get(e.e).copied().is_some_and(char::is_alphanumeric);
    if d == "*" && out.ends_with('*') && !out.ends_with("\\*") && !after_alnum {
        "_"
    } else {
        d
    }
}

/// Render `chars[lo..hi]` with the marks that fall into the window, written as `how` says.
pub(in crate::markdown::export) fn sweep(
    chars: &[char],
    marks: &[&Mark],
    (lo, hi): (usize, usize),
    cx: Ctx,
    how: Variant,
) -> String {
    let ems = plan(marks, lo, hi, cx.opts);
    let chars = &chars[lo..hi];
    let n = chars.len();
    let idx = Index::new(&ems);
    let facts = Facts::of(chars);
    let mut ds: Vec<&str> = ems
        .iter()
        .enumerate()
        .map(|(k, e)| delimiter(marks[e.m], chars, e, punct_runs(&ems, &idx, marks, k)))
        .collect();
    let mut out = String::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into ems
    let mut opened = vec![false; ems.len()];
    let mut link_depth = 0usize; // open links / images on the stack
    let mut p = 0usize;
    loop {
        // last char of a closer written at this position, and `out.len()` right after it
        let mut after_close: Option<(char, usize)> = None;
        // closes (marks above a closing one are closed and re-opened)
        while let Some(j) = stack.iter().position(|&i| ems[i].e <= p) {
            let above: Vec<usize> = stack.split_off(j);
            for &i in above.iter().rev() {
                let c = close_str(marks[ems[i].m], ds[i]);
                out.push_str(&c);
                after_close = c.chars().last().map(|c| (c, out.len()));
            }
            for &i in above.iter().skip(1) {
                if ems[i].e > p {
                    out.push_str(&open_str(marks[ems[i].m], ds[i]));
                    stack.push(i);
                }
            }
            link_depth = stack.iter().filter(|&&i| is_link(marks[ems[i].m])).count();
        }
        let run = Index::starting_at(&ems, p);
        // points
        for k in run.clone() {
            if marks[ems[k].m].is_point() && !opened[k] {
                opened[k] = true;
                out.push_str(&point_str(marks[ems[k].m]));
            }
        }
        // opens, longest first
        let mut atom_end = None;
        let mut cands: Vec<usize> = run
            .filter(|&k| !opened[k] && !marks[ems[k].m].is_point())
            .collect();
        cands.sort_by_key(|&k| (std::cmp::Reverse(ems[k].e), ems[k].atomic));
        for k in cands {
            opened[k] = true;
            let e = &ems[k];
            let m = marks[e.m];
            if e.atomic {
                if atom_end.is_none() {
                    let mut raw: String = chars[e.s..e.e].iter().collect();
                    if cx.table {
                        raw = raw.replace('|', "\\|");
                    }
                    out.push_str(&atomic_str(m, &raw));
                    atom_end = Some(e.e);
                }
            } else {
                if !how.keep_star {
                    ds[k] = apart_from_star(ds[k], &out, chars, e);
                }
                out.push_str(&open_str(m, ds[k]));
                if is_link(m) {
                    link_depth += 1;
                }
                stack.push(k);
            }
        }
        if let Some(end) = atom_end {
            p = end;
            continue;
        }
        if p >= n {
            break;
        }
        let raw = how.raw_delimiter_char(
            chars[p],
            &ems,
            Index::starting_at(&ems, p + 1),
            &ds,
            after_close.filter(|a| a.1 == out.len()),
        );
        let esc = Esc {
            cx,
            in_link: link_depth > 0,
            raw,
            all: how.escape_all,
            facts: &facts,
        };
        escape_into(&mut out, chars, p, &esc);
        p += 1;
    }
    // anything left open (shouldn't be): close
    while let Some(i) = stack.pop() {
        out.push_str(&close_str(marks[ems[i].m], ds[i]));
    }
    if cx.heading && out.ends_with('#') {
        let l = out.len() - 1;
        out.insert(l, '\\');
    }
    out
}

fn is_link(m: &Mark) -> bool {
    matches!(m.kind, MarkKind::Link | MarkKind::Image)
}
