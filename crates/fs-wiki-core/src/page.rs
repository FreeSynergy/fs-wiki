//! A page of texts: the query, the place a page starts after, and the page.
//!
//! Pattern: Value object — small immutable values compared by value. A [`Place`] is where a text
//! stands in the order of a listing (the sort key of its path, its name, its id), not how many texts
//! come before it: the position of a text moves when another is added or removed, a place does not.

use fs_store::ObjectId;

use crate::texts::TextEntry;

/// Which texts a page is made of. Both are read in path order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextQuery {
    /// The texts whose path starts with this prefix (exact, case sensitive; empty = all).
    Prefix(String),
    /// The texts whose name, path or text contains this term, ignoring case.
    Term(String),
}

/// Where a text stands in the order of a listing: its sort key (the path as the store sorts it),
/// then its name, then its id. Two texts never have the same place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The sort key of the path (`fs_store::sort_key`).
    pub key: String,
    /// The envelope name of the text.
    pub name: String,
    /// The id of the text.
    pub id: ObjectId,
}

/// A text with its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// The text.
    pub entry: TextEntry,
    /// Where it stands.
    pub place: Place,
}

/// One page of texts, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPage {
    /// At most the size asked for.
    pub items: Vec<Placed>,
    /// Whether texts follow after the last one.
    pub more: bool,
}
