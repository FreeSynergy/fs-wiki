//! The Markdown format adapter objects.
//!
//! Pattern: Adapter — small structs implementing [`Importer`] / [`Exporter`] over the free
//! functions, so Markdown can be passed wherever a format is expected.

use super::export::{export_blocks, export_markdown, ExportOptions};
use super::import::import_markdown;
use crate::doc::{Block, Document};
use crate::format::{Exporter, Importer};

/// Reads Markdown (CommonMark + GFM + wiki links + callouts).
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownImporter;

impl Importer for MarkdownImporter {
    fn import(&self, src: &str) -> Document {
        import_markdown(src)
    }
}

/// Writes Markdown with the given [`ExportOptions`].
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownExporter {
    /// Export options.
    pub options: ExportOptions,
}

impl Exporter for MarkdownExporter {
    fn export(&self, doc: &Document) -> String {
        export_markdown(doc, self.options)
    }
    fn export_blocks(&self, blocks: &[Block]) -> String {
        export_blocks(blocks, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_round_trip_through_the_traits() {
        let doc = MarkdownImporter.import("# T\n\nsome *text*\n");
        assert_eq!(
            MarkdownExporter::default().export(&doc),
            "# T\n\nsome *text*\n"
        );
        assert_eq!(
            MarkdownExporter::default().export_blocks(&doc.blocks[1..]),
            "some *text*\n"
        );
    }
}
