//! Search with snippets and filters (`text.search`).
//!
//! Pattern: Value object — the query, the hit and its snippets are plain data. A [`SearchPlace`] is the
//! tuple (relevance descending, updated descending, id ascending) a page continues after.

use fs_objects::{ObjectId, Timestamp};

/// A filter that includes, excludes or ignores (`Tri`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tri {
    /// No restriction.
    #[default]
    Any,
    /// Only these.
    Only,
    /// Not these.
    Exclude,
}

/// The filters of a search; all must hold (`SearchFilter`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchFilter {
    /// Only texts below this space or folder.
    pub space: Option<ObjectId>,
    /// Only texts whose path starts with this (exact, case sensitive); empty = any.
    pub path_prefix: String,
    /// Templates.
    pub template: Tri,
    /// Redirects.
    pub redirect: Tri,
    /// Last writer, compared exactly; empty = any.
    pub author: String,
    /// Updated at or after, in milliseconds; 0 = no restriction.
    pub updated_from_ms: i64,
    /// Updated before (half open), in milliseconds; 0 = no restriction.
    pub updated_to_ms: i64,
}

/// A search (`SearchQuery`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery {
    /// Words, all must match whole, case-insensitive; empty = filters only.
    pub term: String,
    /// The last word matches as a prefix.
    pub prefix: bool,
    /// The filters.
    pub filter: SearchFilter,
}

/// A range of a snippet in UTF-8 bytes, half open, on character boundaries (`Highlight`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    /// First byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

/// A piece of a text with the matches in it (`Snippet`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snippet {
    /// `name`, `path` or `body`.
    pub field: &'static str,
    /// At most `MAX_SNIPPET_BYTES`.
    pub text: String,
    /// Ordered, not overlapping.
    pub highlights: Vec<Highlight>,
    /// The text continues before this snippet.
    pub cut_before: bool,
    /// The text continues after it.
    pub cut_after: bool,
}

/// One found text (`SearchHit`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    /// The text.
    pub id: ObjectId,
    /// Its path.
    pub path: String,
    /// Derived name.
    pub name: String,
    /// When it was last written.
    pub updated: Timestamp,
    /// Who wrote it last.
    pub updated_by: String,
    /// Is a template.
    pub template: bool,
    /// Is a redirect.
    pub redirect: bool,
    /// At most three; none for a filters-only search.
    pub snippets: Vec<Snippet>,
    /// How well it matched (not on the wire; it orders the hits).
    pub relevance: u32,
}

impl SearchHit {
    /// Where this hit stands in the order of the result.
    pub fn place(&self) -> SearchPlace {
        SearchPlace {
            relevance: self.relevance,
            updated_ms: self.updated.timestamp_millis(),
            id: self.id,
        }
    }
}

/// Where a hit stands: relevance descending, updated descending, id ascending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchPlace {
    /// Relevance of the hit.
    pub relevance: u32,
    /// Update time in milliseconds.
    pub updated_ms: i64,
    /// The id.
    pub id: ObjectId,
}

/// A page of hits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPage {
    /// At most the size asked for.
    pub items: Vec<SearchHit>,
    /// Whether hits follow.
    pub more: bool,
}
