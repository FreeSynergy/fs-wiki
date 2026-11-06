//! The heading of a callout.
//!
//! Pattern: Adapter — fs-text keeps a callout title as Markdown as written (`A **bold** word`).
//! It is read as Markdown here and shown with its formatting, never as markup; the whole title
//! is strong, like the type shown when there is no title. A title that is more than one line of
//! inline text (a block, a list) is shown as its plain text.

use fs_components_widgets::{Inline, Mark, MarkKind};
use fs_text::{BlockKind, import_markdown};

use super::inline::inline_of;

/// The heading line for a callout of type `ty` with Markdown title `title`.
pub(super) fn heading(ty: &str, title: &str) -> Inline {
    let inline = if title.trim().is_empty() {
        Inline::plain(ty)
    } else {
        let doc = import_markdown(title);
        match doc.blocks.as_slice() {
            [one] if matches!(one.attrs.kind, BlockKind::Paragraph) => inline_of(one),
            blocks => Inline::plain(
                blocks
                    .iter()
                    .map(|b| b.text.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        }
    };
    let len = inline.text.chars().count();
    let mut marks = inline.marks;
    marks.push(Mark::new(MarkKind::Strong, 0, len));
    Inline::new(inline.text, marks)
}
