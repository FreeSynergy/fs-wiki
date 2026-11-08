//! The result of following redirects (`text.redirect`).
//!
//! Pattern: Value object — plain data. These are results, not errors.

use crate::texts::TextEntry;

/// Most redirects `resolve` follows (`maxRedirectHops`).
pub const MAX_REDIRECT_HOPS: usize = 8;

/// How a chain of redirects ended (`Outcome` without `unknown`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The end of the chain is a text that is no redirect.
    Resolved,
    /// The start or a target does not exist.
    NotFound,
    /// A path came up twice; wins whenever a cycle is seen.
    Loop,
    /// More than [`MAX_REDIRECT_HOPS`] redirects and no cycle seen.
    TooManyHops,
}

/// What `resolve` found (`Resolution`).
#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    /// How it ended.
    pub outcome: Outcome,
    /// The paths visited, start first. For `Resolved` the final path is not in it.
    pub hops: Vec<String>,
    /// The end of the chain; only for `Resolved`.
    pub text: Option<TextEntry>,
}
