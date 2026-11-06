//! Links inside a callout title.
//!
//! Pattern: Adapter — a callout keeps its title as Markdown as written (`> [!note] See
//! [x](a/b.md)`), not as marks. The title is read with the Markdown parser of fs-text, so only
//! real links count (`[x](a)`, `[[a|x]]`, images); a link inside a code span, an escaped bracket
//! or loose text is not one. The rewrite after a move is made on that small document with the
//! same rule as for a page (`relink.rs`); a title with no real change to a link stays byte for
//! byte what it was.

use fs_text::{export_markdown, import_markdown, Block, Container, Document, ExportOptions};
use fs_text::{LinkValue, MarkKind};

use super::relink::relinked;
use super::shift::Shift;

/// The titles of the callouts `block` stands in.
pub(super) fn titles_of(block: &Block) -> impl Iterator<Item = &str> {
    block.attrs.containers.iter().filter_map(|c| match c {
        Container::Callout { title, .. } => Some(title.as_str()),
        _ => None,
    })
}

/// The real links of a title.
pub(super) fn title_links(title: &str) -> Vec<LinkValue> {
    import_markdown(title)
        .blocks
        .iter()
        .flat_map(|b| {
            b.marks
                .iter()
                .filter(|m| matches!(m.kind, MarkKind::Link | MarkKind::Image))
                .filter_map(|m| m.link().cloned())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The real links of the callout titles of `block`.
pub(super) fn links_in_titles(block: &Block) -> Vec<LinkValue> {
    titles_of(block).flat_map(title_links).collect()
}

/// `title` for a page that moves from `before` to `after`, with its links rewritten; `None` if
/// no link changes.
pub(super) fn relinked_title(
    title: &str,
    before: &str,
    after: &str,
    shift: &Shift,
) -> Option<String> {
    let doc: Document = import_markdown(title);
    let new = relinked(&doc, before, after, shift)?;
    let text = export_markdown(&new, ExportOptions::default());
    Some(text.trim_end_matches('\n').to_string())
}
