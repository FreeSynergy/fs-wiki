//! Revisions of a text and what a write expects (`text.history`, `text.editSummary`).
//!
//! Pattern: Value object — plain data, compared by value. `number` counts the writes of one text from 1
//! and is never reused; it is the only thing here that orders revisions (`TextEntry::revision` does
//! not: it is random).

use fs_objects::{ObjectId, Timestamp};
use fs_text::Document;

/// What a write expects of the text (`Expect` in `texts.capnp`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Expect {
    /// Write whatever the text is now.
    #[default]
    Any,
    /// Only if the text still has this `TextEntry::revision`, else `Error::Conflict`.
    Revision(u64),
}

/// One revision without its document (`RevisionInfo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionInfo {
    /// The text.
    pub text: ObjectId,
    /// 1 = the creation; gaps are revisions removed by `purge_revision`.
    pub number: u64,
    /// The `TextEntry::revision` this write produced.
    pub revision: u64,
    /// Actor of the write; empty = not known.
    pub author: String,
    /// When it was written.
    pub at: Timestamp,
    /// Bytes of the document JSON of this revision.
    pub size: u64,
    /// What the writer said, else the provider's own note (`created`, `moved`, ...).
    pub summary: String,
    /// The path of the text at that time.
    pub path: String,
    /// The number this revision was restored from; 0 = not a restore.
    pub restored_from: u64,
}

/// One revision with its document (`RevisionContent`).
#[derive(Clone, Debug, PartialEq)]
pub struct RevisionContent {
    /// What is known about it.
    pub info: RevisionInfo,
    /// The document as it was.
    pub document: Document,
}

/// A page of revisions, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionPage {
    /// At most the size asked for.
    pub items: Vec<RevisionInfo>,
    /// Whether older revisions follow after the last one.
    pub more: bool,
}
