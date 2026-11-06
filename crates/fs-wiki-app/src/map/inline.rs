//! Marks of one block to inline marks.
//!
//! Pattern: Adapter (Strategy by mark kind) — kinds the components know become marks; the rest
//! leave their text as it is. A footnote reference (a point) gets its label written into the
//! text so it is not lost.

use fs_components_widgets::{Inline, Mark, MarkKind};
use fs_text::{Block, MarkKind as Kind, MarkValue};

/// The inline text of a block.
pub(super) fn inline_of(block: &Block) -> Inline {
    shaped(&block.text.to_string(), block, 0)
}

/// The text `text` (a part of the block starting at char `from`) with the block's marks that
/// fall into it, shifted; footnote references are written out as `[^label]`.
pub(super) fn shaped(text: &str, block: &Block, from: usize) -> Inline {
    let len = text.chars().count();
    let to = from + len;
    // A soft break (a newline inside a paragraph) is a space in Markdown; a hard break (stored as
    // U+2028) is a line break. One character for one, so the offsets of the marks stay valid.
    let mut chars: Vec<char> = text
        .chars()
        .map(|c| match c {
            '\n' => ' ',
            fs_text::markdown::HARD_BREAK => '\n',
            other => other,
        })
        .collect();
    let mut marks: Vec<Mark> = Vec::new();
    for m in &block.marks {
        if let (Kind::FootnoteRef, MarkValue::Name(label)) = (m.kind, &m.value) {
            if m.start >= from && m.start <= to {
                insert_at(
                    &mut chars,
                    &mut marks,
                    m.start - from,
                    &format!("[^{label}]"),
                );
            }
            continue;
        }
        let Some(kind) = kind_of(m) else { continue };
        let (start, end) = (m.start.max(from), m.end.min(to));
        if start < end {
            marks.push(Mark::new(kind, start - from, end - from));
        }
    }
    Inline::new(chars.into_iter().collect::<String>(), marks)
}

/// The component mark of a `fs-text` mark, if the components know the kind.
fn kind_of(m: &fs_text::Mark) -> Option<MarkKind> {
    match (m.kind, &m.value) {
        (Kind::Strong, _) => Some(MarkKind::Strong),
        (Kind::Emphasis, _) => Some(MarkKind::Emphasis),
        (Kind::Strike, _) => Some(MarkKind::Strike),
        (Kind::Code, _) => Some(MarkKind::Code),
        (Kind::Link, MarkValue::Link(l)) => Some(MarkKind::Link { href: href_of(l) }),
        _ => None,
    }
}

/// Write `inserted` into the text at `at` and move the marks behind it.
fn insert_at(chars: &mut Vec<char>, marks: &mut [Mark], at: usize, inserted: &str) {
    let n = inserted.chars().count();
    let at = at.min(chars.len());
    chars.splice(at..at, inserted.chars());
    for m in marks.iter_mut() {
        if m.start >= at {
            m.start += n;
        }
        if m.end > at {
            m.end += n;
        }
    }
}

/// The address the reader hands back when the link is followed. `[[a/b]]` is a path from the root
/// (that is how the core reads it), so it gets a leading `/`; a Markdown link stays as written and
/// is relative to its page.
fn href_of(link: &fs_text::LinkValue) -> String {
    let wiki = matches!(
        link.style,
        fs_text::LinkStyle::Wiki | fs_text::LinkStyle::Embed
    );
    if wiki && link.is_internal() && !link.path.starts_with(['#', '/']) {
        format!("/{}", link.path)
    } else {
        link.path.clone()
    }
}
