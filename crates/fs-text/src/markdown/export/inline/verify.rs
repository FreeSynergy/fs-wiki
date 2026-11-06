//! Inline rendering that checks itself.
//!
//! Pattern: Generate and test — CommonMark pairs delimiter runs by flanking and the "rule of
//! three", which no local escaping rule predicts. The text is written plainly first; if
//! reading it back does not give the same text and emphasis, other [`Variant`]s are tried and
//! the first one that reads back right wins. Nothing changes for text that already reads back.

use super::ctx::Ctx;
use super::emit::sweep;
use super::plan::plan;
use super::variant::Variant;
use crate::doc::BlockKind;
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::import_markdown;

/// An emphasis or strong span: start, end, strong?, written with `_`?
type Span = (usize, usize, bool, bool);

fn span_of(kind: MarkKind, s: usize, e: usize, value: &MarkValue) -> Option<Span> {
    let strong = match kind {
        MarkKind::Emphasis => false,
        MarkKind::Strong => true,
        _ => return None,
    };
    Some((s, e, strong, *value == MarkValue::Name("_".into())))
}

fn reads_back(out: &str, text: &str, want: &[Span]) -> bool {
    let d = import_markdown(out);
    let [b] = &d.blocks[..] else {
        return false;
    };
    if b.attrs.kind != BlockKind::Paragraph || b.text.to_string() != text {
        return false;
    }
    let mut got: Vec<Span> = b
        .marks
        .iter()
        .filter_map(|m| span_of(m.kind, m.start, m.end, &m.value))
        .collect();
    got.sort_unstable();
    got == want
}

/// Render `chars[lo..hi]` with the marks that fall into the window.
pub(in crate::markdown::export) fn inline(
    chars: &[char],
    marks: &[&Mark],
    lo: usize,
    hi: usize,
    cx: Ctx,
) -> String {
    let first = sweep(chars, marks, (lo, hi), cx, Variant::default());
    let text: String = chars[lo..hi].iter().collect();
    let mut want: Vec<Span> = plan(marks, lo, hi, cx.opts)
        .iter()
        .filter(|e| !e.atomic)
        .filter_map(|e| span_of(marks[e.m].kind, e.s, e.e, &marks[e.m].value))
        .collect();
    want.sort_unstable();
    if want.is_empty() || reads_back(&first, &text, &want) {
        return first;
    }
    Variant::alternatives()
        .map(|v| sweep(chars, marks, (lo, hi), cx, v))
        .find(|s| reads_back(s, &text, &want))
        .unwrap_or(first)
}
