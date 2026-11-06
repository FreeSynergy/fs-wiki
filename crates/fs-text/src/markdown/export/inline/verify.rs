//! Writing a block so that it reads back as the same block.
//!
//! Pattern: Generate and test — CommonMark pairs delimiter runs by flanking and the "rule of
//! three", and text can look like other syntax (a table, a definition, an image), which no
//! local escaping rule predicts. The whole block is written plainly first and read back; if it
//! does not read back as the same text and kind, other [`Variant`]s are tried, and when none
//! works the form that escapes everything is used, never a form known to be wrong. Marks that
//! cannot be written (crossing ones) make a form second best, not wrong. Nothing changes for
//! blocks that already read back.

use super::variant::Variant;
use crate::doc::Block;
use crate::mark::{Mark, MarkKind, MarkValue};
use crate::markdown::{import_plain, ExportOptions};

/// A mark as the read-back compares it: (kind, start, end, value key).
type Norm = (MarkKind, usize, usize, String);

fn representable(m: &Mark) -> bool {
    match m.kind {
        MarkKind::Strong | MarkKind::Emphasis | MarkKind::Strike | MarkKind::Code => {
            m.end > m.start
        }
        MarkKind::Link | MarkKind::Image | MarkKind::Html => true,
        MarkKind::FootnoteRef => m.is_point(),
        _ => false,
    }
}

/// Marks reduced to what the Markdown says: touching spans of one emphasis kind are one span, `*` and `_` are the same,
/// links compare path, title and style.
fn normal(marks: &[&Mark]) -> Vec<Norm> {
    let merge = |k: MarkKind| matches!(k, MarkKind::Strong | MarkKind::Emphasis | MarkKind::Strike);
    let key = |m: &Mark| match &m.value {
        MarkValue::Link(l) => format!("{}\u{1}{:?}\u{1}{:?}", l.path, l.title, l.style),
        MarkValue::Name(n) if m.kind == MarkKind::FootnoteRef => n.clone(),
        _ => String::new(),
    };
    let norm = |m: &&Mark| (m.kind, m.start, m.end, key(m));
    // emphasis spans: touching ones of a kind are one; others keep their order (a link around
    // an empty image is not an empty image around a link)
    let mut spans: Vec<Norm> = marks
        .iter()
        .filter(|m| representable(m) && merge(m.kind))
        .map(norm)
        .collect();
    spans.sort();
    let mut out: Vec<Norm> = Vec::with_capacity(spans.len());
    for s in spans {
        match out.last_mut() {
            Some(l) if l.0 == s.0 && s.1 <= l.2 => l.2 = l.2.max(s.2),
            _ => out.push(s),
        }
    }
    let mut rest: Vec<Norm> = marks
        .iter()
        .filter(|m| representable(m) && !merge(m.kind))
        .map(norm)
        .collect();
    rest.sort_by_key(|n| (n.1, std::cmp::Reverse(n.2)));
    out.sort_by_key(|n| (n.0 as u8, n.1));
    out.extend(rest);
    out
}

/// How well a written form reads back.
#[derive(PartialEq, PartialOrd)]
enum Verdict {
    /// Another block, or other text.
    Wrong,
    /// The same block and text; some marks differ.
    Text,
    /// The same block, text and marks.
    Exact,
}

fn read_back(b: &Block, lines: &[String]) -> Verdict {
    let d = import_plain(&lines.join("\n"));
    let [got] = &d.blocks[..] else {
        return Verdict::Wrong;
    };
    let (a, g) = (&b.attrs, &got.attrs);
    if a.kind != g.kind
        || !g.containers.is_empty()
        || a.block_ref != g.block_ref
        || a.heading_attrs != g.heading_attrs
        || got.text != b.text
    {
        return Verdict::Wrong;
    }
    let want: Vec<&Mark> = b.marks.iter().collect();
    let have: Vec<&Mark> = got.marks.iter().collect();
    if normal(&want) == normal(&have) {
        Verdict::Exact
    } else {
        Verdict::Text
    }
}

/// Is a read-back worth doing? An empty block has nothing to check.
fn checkable(b: &Block) -> bool {
    !b.text.is_empty() && !b.attrs.extra.contains_key("callout-header")
}

/// The lines of block `b` in the first form that reads back right. `render` writes the block in
/// a given [`Variant`].
pub(in crate::markdown::export) fn choose(
    b: &Block,
    opts: ExportOptions,
    render: &dyn Fn(Variant) -> Vec<String>,
) -> Vec<String> {
    let first = render(Variant::default());
    if opts.layers || !checkable(b) {
        return first;
    }
    let mut best = match read_back(b, &first) {
        Verdict::Exact => return first,
        v => (v, first),
    };
    // the variants only change how delimiter runs are written
    let delimiters = b
        .marks
        .iter()
        .any(|m| matches!(m.kind, MarkKind::Strong | MarkKind::Emphasis));
    if delimiters {
        for v in Variant::alternatives() {
            let lines = render(v);
            match read_back(b, &lines) {
                Verdict::Exact => return lines,
                x if x > best.0 => best = (x, lines),
                _ => {}
            }
        }
    }
    if best.0 == Verdict::Wrong {
        return render(Variant::everything());
    }
    best.1
}
