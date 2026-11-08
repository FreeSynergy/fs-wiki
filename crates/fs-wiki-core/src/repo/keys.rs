//! Names of the index fields.
//!
//! Pattern: none (constants).

/// The path as written: exact lookups and prefix ranges.
pub(super) const PATH: &str = "path";
/// The sort key of the path (`fs_objects::sort_key`): what pages are ordered by, so that a condition
/// on the raw value (binary order) and the order of the store (the same key) agree.
pub(super) const PATH_KEY: &str = "path_key";
/// The id as text: the last tie-break of a page, compared as text like the store orders by it.
pub(super) const ID_KEY: &str = "id_key";
/// The plain text of each block: what `find` searches.
pub(super) const BLOCK: &str = "block";
/// The paths every internal link can mean: what `backlinks` searches.
pub(super) const LINK: &str = "link";
