//! The [`Texts`] contract: what the wiki program offers to everyone else.
//!
//! Pattern: Repository (interface) + Null Object — callers depend on this trait, never on the
//! store-backed `WikiRepo`. It is the Rust form of `Texts` in `texts.capnp` (fs-interfaces). Like
//! a parent class (ADR 0045) every provider has every method: one it does not override answers
//! an explicit [`Unsupported`], and `capabilities` reports the same.

use async_trait::async_trait;
use fs_interfaces::{Capabilities, Unsupported};
use fs_store::{Envelope, ObjectId};
use fs_text::Document;

use crate::error::Result;

/// The explicit "not possible" of a method a provider does not override.
pub(crate) fn not_possible<T>(capability: &str) -> Result<T> {
    Err(Unsupported::new(capability, "this provider does not offer it").into())
}

/// One text: its AS2 envelope, readable path and document (`TextEntry` in `texts.capnp`).
#[derive(Clone, Debug, PartialEq)]
pub struct TextEntry {
    /// Object id (also the id of the document).
    pub id: ObjectId,
    /// AS2 `Article` envelope.
    pub envelope: Envelope,
    /// Readable path, unique.
    pub path: String,
    /// The document: text + layers.
    pub document: Document,
}

/// One row of the link table (`LinkRow` in `texts.capnp`, plus what a caller needs to show it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkEntry {
    /// The text that contains the link.
    pub source: ObjectId,
    /// Readable path of the target, as written (may carry a `#fragment`).
    pub target: String,
    /// Where in the source: `<block id>:<start>-<end>` (char offsets of the link text).
    pub at: String,
    /// The link text as shown.
    pub title: String,
    /// Whether the link points into the wiki (not an URL or mail address).
    pub internal: bool,
}

/// One page of a list: the Rust form of the contract's cursor (an offset behind it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Paging {
    /// Maximum number of texts; `None` = all.
    pub limit: Option<usize>,
    /// Texts to skip.
    pub offset: usize,
}

impl Paging {
    /// A page of `limit` texts after skipping `offset`.
    #[must_use]
    pub fn new(limit: usize, offset: usize) -> Self {
        Self {
            limit: Some(limit),
            offset,
        }
    }
}

/// Texts: pages with layers, found by path or term, linked to each other.
///
/// Interface status: draft (ADR 0045).
#[async_trait]
pub trait Texts: Send + Sync {
    /// What this provider can do, per capability id.
    fn capabilities(&self) -> Capabilities;
    /// `text.read`: the text, if it exists.
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>>;
    /// `text.read`: lookup by readable path.
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>>;
    /// `text.read`: texts whose path starts with `prefix` (empty = all), in path order, paged.
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>>;

    /// `text.create`: a new text at `path` from Markdown. A taken path is an error.
    async fn create(&self, _path: &str, _markdown: &str) -> Result<TextEntry> {
        not_possible("text.create")
    }
    /// `text.edit`: replace the document of `id` (the id of the document is forced to `id`).
    async fn edit(&self, _id: ObjectId, _document: Document) -> Result<TextEntry> {
        not_possible("text.edit")
    }
    /// `text.move`: give `id` a new path; links to the old path in other texts follow.
    async fn move_to(&self, _id: ObjectId, _path: &str) -> Result<TextEntry> {
        not_possible("text.move")
    }
    /// `text.delete`: delete a text with its layers (undoable through the store).
    async fn delete(&self, _id: ObjectId) -> Result<()> {
        not_possible("text.delete")
    }
    /// `text.find`: texts whose name, path or text contains `term`, ignoring case.
    async fn find(&self, _term: &str, _page: Paging) -> Result<Vec<TextEntry>> {
        not_possible("text.find")
    }
    /// `link.read`: the links inside `id`.
    async fn links(&self, _id: ObjectId) -> Result<Vec<LinkEntry>> {
        not_possible("link.read")
    }
    /// `link.read`: the links in all texts that point at `path`.
    async fn backlinks(&self, _path: &str) -> Result<Vec<LinkEntry>> {
        not_possible("link.read")
    }
    /// `text.import`: `text` of `media_type` is the file of `path`. A new path creates a text; an
    /// existing one is an outside change: the difference becomes a normal edit and layers stay.
    async fn import_text(&self, _path: &str, _media_type: &str, _text: &str) -> Result<TextEntry> {
        not_possible("text.import")
    }
    /// `text.export`: the text as a file of `media_type` (layers are not written).
    async fn export_text(&self, _id: ObjectId, _media_type: &str) -> Result<String> {
        not_possible("text.export")
    }
}
