//! Media type to format: the choice behind `importText` / `exportText` of `texts.capnp`.
//!
//! Pattern: Factory — a media type string picks the format; an unknown one is an error, never a
//! guess. Parameters (`; charset=utf-8`) and case are ignored, as in HTTP. Markdown is the one
//! file format today; others plug in behind `fs_text::Importer` / `Exporter`.

use fs_text::{export_markdown, import_markdown, Document, ExportOptions};

use crate::error::{Error, Result};

/// Media type of Markdown.
pub const MARKDOWN: &str = "text/markdown";

/// The text formats a wiki reads and writes by media type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFormat {
    /// Markdown (`text/markdown`).
    Markdown,
}

impl TextFormat {
    /// The format for `media_type`.
    ///
    /// # Errors
    /// [`Error::MediaType`] when no format is known for it.
    pub fn from_media_type(media_type: &str) -> Result<Self> {
        let essence = media_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match essence.as_str() {
            MARKDOWN => Ok(Self::Markdown),
            _ => Err(Error::MediaType(media_type.to_string())),
        }
    }

    /// Read `text` into a block document.
    #[must_use]
    pub fn import(self, text: &str) -> Document {
        match self {
            Self::Markdown => import_markdown(text),
        }
    }

    /// Write a document as a file of this format. Layers are not written (ADR 0042).
    #[must_use]
    pub fn export(self, doc: &Document) -> String {
        match self {
            Self::Markdown => export_markdown(doc, ExportOptions::default()),
        }
    }

    /// Plan and apply an outside change of the file `text` to `doc`; true when anything changed.
    pub fn import_external(self, doc: &mut Document, text: &str) -> bool {
        match self {
            Self::Markdown => !fs_text::import_external(doc, text).0.is_empty(),
        }
    }
}
