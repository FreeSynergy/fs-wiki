//! Error type of the wiki core.
//!
//! Pattern: none (plain error enum via `thiserror`) — one `Error` for every public operation,
//! with `From` conversions for the store and JSON layers.

use fs_store::ObjectId;

/// Result of every fallible operation of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong in the wiki core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The object store failed.
    #[error("object store: {0}")]
    Store(#[from] fs_store::Error),
    /// No text with this id.
    #[error("text {0} not found")]
    NotFound(ObjectId),
    /// Another text already has this path.
    #[error("path {0:?} is taken")]
    PathTaken(String),
    /// The path is not a readable path (empty, empty segment, `#`, `..`, control characters).
    #[error("invalid path {path:?}: {reason}")]
    InvalidPath {
        /// The path as given.
        path: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A folder move that cannot be done at all, whatever the store holds.
    #[error("cannot move {from:?} to {to:?}: {reason}")]
    InvalidMove {
        /// The folder as given.
        from: String,
        /// The target as given.
        to: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// No text at the path and none below it.
    #[error("nothing at {0:?} or below it")]
    NoSuchFolder(String),
    /// No import / export format is known for this media type.
    #[error("unknown media type {0:?} (known: text/markdown)")]
    MediaType(String),
    /// The JSON of a document or a stored record could not be read or written.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A stored object is not a text of this wiki.
    #[error("object {0} is not a wiki text")]
    NotAText(ObjectId),
    /// The explicit "not possible" of a provider (ADR 0045).
    #[error(transparent)]
    Unsupported(#[from] fs_interfaces::Unsupported),
}
