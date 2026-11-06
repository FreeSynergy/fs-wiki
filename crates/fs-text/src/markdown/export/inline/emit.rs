//! The inline sweep: walks the text once, opening and closing marks.
//!
//! Pattern: Strategy client — what a mark writes is decided by its [`MarkWriter`]; this code
//! only keeps the open-mark stack consistent (re-opening marks that were closed early by an
//! overlapping neighbour). Spans are looked up by position (sorted by start, `Index` by end),
//! so the sweep is linear in text plus marks.

use super::atomic::{atomic_str, point_str};
use super::ctx::Ctx;
use super::escape::{escape_into, Esc, Facts};
use super::plan::{plan, Em, Index};
use super::runs::{apart_from_star, delimiter, punct_runs};
use super::variant::Variant;
use crate::mark::{Mark, MarkKind};
use crate::markdown::{md_syntax, MarkWriter};

fn open_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).open(d)
}

fn close_str(m: &Mark, d: &str) -> String {
    md_syntax(m.kind).close(m, d)
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
    let facts = Facts::of(
        chars,
        ems.iter().any(|e| marks[e.m].kind == MarkKind::Strike),
    );
    let mut ds: Vec<&str> = ems
        .iter()
        .enumerate()
        .map(|(k, e)| delimiter(marks[e.m], chars, e, punct_runs(&ems, &idx, marks, k)))
        .collect();
    let mut out = String::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into ems
    let mut opened = vec![false; ems.len()];
    let mut p = 0usize;
    loop {
        let after_close = close_ended(&mut out, &mut stack, (&ems, marks, &ds), p);
        let mut link_depth = stack.iter().filter(|&&i| is_link(marks[ems[i].m])).count();
        let run = Index::starting_at(&ems, p);
        write_points(&mut out, run.clone(), (&ems, marks), &mut opened);
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
            form: how.form,
            edge_next: chars[p] == '\\' && idx.touching(&ems, p + 1).next().is_some(),
            facts: &facts,
        };
        escape_into(&mut out, chars, p, &esc);
        p += 1;
    }
    // anything left open (shouldn't be): close
    while let Some(i) = stack.pop() {
        out.push_str(&close_str(marks[ems[i].m], ds[i]));
    }
    // a closing run of `#` would end the heading: escape the last one (unless it already is)
    if cx.heading && out.ends_with('#') {
        let slashes = out[..out.len() - 1]
            .chars()
            .rev()
            .take_while(|c| *c == '\\')
            .count();
        if slashes % 2 == 0 {
            let l = out.len() - 1;
            out.insert(l, '\\');
        }
    }
    out
}

/// Close the marks that end at `p` (marks above a closing one are closed and re-opened).
/// Returns the last char of a closer written and `out.len()` right after it.
fn close_ended(
    out: &mut String,
    stack: &mut Vec<usize>,
    (ems, marks, ds): (&[Em], &[&Mark], &[&str]),
    p: usize,
) -> Option<(char, usize)> {
    let mut after_close = None;
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
    }
    after_close
}

/// Points at one place (footnote references, variables, links and images without text).
fn write_points(
    out: &mut String,
    run: std::ops::Range<usize>,
    (ems, marks): (&[Em], &[&Mark]),
    opened: &mut [bool],
) {
    let mut nested: Vec<&Mark> = Vec::new();
    for k in run {
        let m = marks[ems[k].m];
        if m.is_point() && !opened[k] {
            opened[k] = true;
            if is_link(m) {
                nested.push(m);
            } else {
                out.push_str(&point_str(m));
            }
        }
    }
    out.push_str(&nested_points(&nested));
}

/// Links and images without text at one place: the first wraps the rest (`[![](i.png)](t)`).
fn nested_points(list: &[&Mark]) -> String {
    match list {
        [] => String::new(),
        [one] => point_str(one),
        [first, rest @ ..] => format!(
            "{}{}{}",
            open_str(first, ""),
            nested_points(rest),
            close_str(first, "")
        ),
    }
}

fn is_link(m: &Mark) -> bool {
    matches!(m.kind, MarkKind::Link | MarkKind::Image)
}
