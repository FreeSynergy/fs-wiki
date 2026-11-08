//! The values of the `Texts` contract: a text, a link row, a page request.
//!
//! Pattern: Value object — plain data, compared by value.

use fs_objects::{Envelope, ObjectId};
use fs_text::Document;

/// One text: its AS2 envelope, readable path and document (`TextEntry` in `texts.capnp`).
#[derive(Clone, Debug, PartialEq)]
pub struct TextEntry {
    /// Object id (also the id of the document).
    pub id: ObjectId,
    /// AS2 `Article` envelope.
    pub envelope: Envelope,
    /// Readable path, unique.
    pub path: String,
    /// Changes with every write and is never used twice (not a counter: a store undo cannot make one
    /// come round again); 0 only for records from before revisions.
    pub revision: u64,
    /// The document: text + layers.
    pub document: Document,
    /// Marked as a template (`text.template`).
    pub template: bool,
    /// The path this text forwards to; empty = not a redirect (`text.redirect`).
    pub redirect_to: String,
    /// Actor of the last write; empty = not known.
    pub updated_by: String,
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
