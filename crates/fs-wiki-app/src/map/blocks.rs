//! One block of the document, and the document.
//!
//! Pattern: Strategy by block kind — a `match` on `BlockKind`; everything unknown is plain text.

use fs_components_widgets::{Block as Out, Document, Inline};
use fs_text::{Block, BlockKind, Document as Doc};

use super::inline::inline_of;
use super::nest::nest;
use super::table::table;

/// The document as the components draw it, titled `title`.
pub fn to_document(title: &str, doc: &Doc) -> Document {
    Document {
        title: title.to_string(),
        blocks: nest(&doc.blocks.iter().collect::<Vec<_>>(), 0),
    }
}

/// One block without its containers.
pub(super) fn leaf(block: &Block) -> Out {
    let text = block.text.to_string();
    match &block.attrs.kind {
        BlockKind::Paragraph => Out::Paragraph(inline_of(block)),
        BlockKind::Heading { level } => Out::Heading {
            level: *level,
            text: inline_of(block),
        },
        BlockKind::Code { info } => Out::Code {
            lang: info.clone(),
            text,
        },
        BlockKind::FrontMatter => Out::Code {
            lang: "yaml".into(),
            text,
        },
        BlockKind::ThematicBreak => Out::Rule,
        BlockKind::Table { align } => table(block, align),
        BlockKind::Raw | BlockKind::Html => Out::Plain(text),
    }
}

/// A paragraph of plain text.
pub(super) fn paragraph(text: &str) -> Out {
    Out::Paragraph(Inline::plain(text))
}
