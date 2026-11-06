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
use crate::mark::{Mark, MarkKind};
use crate::markdown::{md_syntax, MarkWriter};

fn delimiter(m: &Mark, chars: &[char], e: &Em) -> &'static str {
    match m.kind {
        MarkKind::Emphasis => delim(m, chars, e.s, e.e, "*", "_"),
        MarkKind::Strong => delim(m, chars, e.s, e.e, "**", "__"),
        _ => "",
    }
}

fn open_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).open(d)
}

fn close_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).close(m, d)
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
    let ds: Vec<&str> = ems
        .iter()
        .map(|e| delimiter(marks[e.m], chars, e))
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
