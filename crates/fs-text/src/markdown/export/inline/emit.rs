//! The inline sweep: walks the text once, opening and closing marks.
//!
//! Pattern: Strategy client — what a mark writes is decided by its [`MarkWriter`]; this code
//! only keeps the open-mark stack consistent (re-opening marks that were closed early by an
//! overlapping neighbour).

use super::atomic::{atomic_str, point_str};
use super::ctx::Ctx;
use super::delim::delim;
use super::escape::escape_into;
use super::plan::{plan, Em};
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::{md_syntax, MarkWriter};

fn delimiter(m: &Mark, chars: &[char], e: &Em, star_follows: bool) -> &'static str {
    match m.kind {
        MarkKind::Emphasis => delim(m, chars, e.s, e.e, "*", "_", star_follows),
        MarkKind::Strong => delim(m, chars, e.s, e.e, "**", "__", star_follows),
        _ => "",
    }
}

/// Does a span that writes `*` open right where `e` ends? A span that keeps `_` does not count:
/// two `_` runs would fuse.
fn star_follows(ems: &[Em], marks: &[&Mark], e: &Em) -> bool {
    ems.iter().any(|o| {
        let m = marks[o.m];
        o.s == e.e
            && o.e > o.s
            && matches!(m.kind, MarkKind::Emphasis | MarkKind::Strong)
            && m.value != MarkValue::Name("_".into())
    })
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

/// Render `chars[lo..hi]` with the marks that fall into the window.
pub(in crate::markdown::export) fn inline(
    chars: &[char],
    marks: &[&Mark],
    lo: usize,
    hi: usize,
    cx: Ctx,
) -> String {
    let ems = plan(marks, lo, hi, cx.opts);
    let chars = &chars[lo..hi];
    let n = chars.len();
    let mut ds: Vec<&str> = ems
        .iter()
        .map(|e| delimiter(marks[e.m], chars, e, star_follows(&ems, marks, e)))
        .collect();
    let mut out = String::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into ems
    let mut opened = vec![false; ems.len()];
    let mut p = 0usize;
    loop {
        // closes (marks above a closing one are closed and re-opened)
        while let Some(j) = stack.iter().position(|&i| ems[i].e <= p) {
            let above: Vec<usize> = stack.split_off(j);
            for &i in above.iter().rev() {
                out.push_str(&close_str(marks[ems[i].m], ds[i]));
            }
            for &i in above.iter().skip(1) {
                if ems[i].e > p {
                    out.push_str(&open_str(marks[ems[i].m], ds[i]));
                    stack.push(i);
                }
            }
        }
        // points
        for (k, e) in ems.iter().enumerate() {
            if e.s == p && marks[e.m].is_point() && !opened[k] {
                opened[k] = true;
                out.push_str(&point_str(marks[e.m]));
            }
        }
        // opens, longest first
        let mut atom_end = None;
        let mut cands: Vec<usize> = (0..ems.len())
            .filter(|&k| !opened[k] && ems[k].s == p && !marks[ems[k].m].is_point())
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
                ds[k] = apart_from_star(ds[k], &out, chars, e);
                out.push_str(&open_str(m, ds[k]));
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
        let in_link = stack
            .iter()
            .any(|&i| matches!(marks[ems[i].m].kind, MarkKind::Link | MarkKind::Image));
        escape_into(&mut out, chars, p, cx, in_link);
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
