//! The bounds of texts 0.2 (`texts.capnp` consts).
//!
//! Pattern: none (constants) — a provider enforces them; a larger request is an invalid one.

/// Longest revision summary (`maxSummaryBytes`).
pub const MAX_SUMMARY_BYTES: usize = 200;
/// Longest search term (`maxSearchTermBytes`).
pub const MAX_SEARCH_TERM_BYTES: usize = 256;
/// Most words in a search term (`maxSearchWords`).
pub const MAX_SEARCH_WORDS: usize = 8;
/// Shortest prefix word of a prefix search (`minPrefixBytes`).
pub const MIN_PREFIX_BYTES: usize = 2;
/// Longest snippet (`maxSnippetBytes`).
pub const MAX_SNIPPET_BYTES: usize = 240;
/// Most snippets of a hit (`maxSnippetsPerHit`).
pub const MAX_SNIPPETS_PER_HIT: usize = 3;
/// Most highlights of a snippet (`maxHighlightsPerSnippet`).
pub const MAX_HIGHLIGHTS: usize = 20;
/// Largest page of `history` (`maxHistoryPage`).
pub const MAX_HISTORY_PAGE: usize = 100;
/// Largest page of `children` (`maxChildrenPage`).
pub const MAX_CHILDREN_PAGE: usize = 200;
/// Largest page of `search` (`maxSearchPage`).
pub const MAX_SEARCH_PAGE: usize = 50;
/// Longest folder name (`maxFolderNameBytes`).
pub const MAX_FOLDER_NAME_BYTES: usize = 255;
