//! The behaviour interface of a mark kind.
//!
//! Pattern: Strategy — editing code asks a [`MarkBehavior`] what to do at a boundary or on
//! overlap instead of matching on the kind; new kinds add an implementation, not a `match` arm.

use serde::{Deserialize, Serialize};

/// What happens when text is typed exactly at a mark boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expand {
    /// Text typed at the start joins the mark.
    Before,
    /// Text typed at the end joins the mark.
    After,
    /// Both boundaries take typed text in.
    Both,
    /// Typed text never joins.
    None,
}

impl Expand {
    /// Does text typed at the start join?
    #[must_use]
    pub fn before(self) -> bool {
        matches!(self, Expand::Before | Expand::Both)
    }
    /// Does text typed at the end join?
    #[must_use]
    pub fn after(self) -> bool {
        matches!(self, Expand::After | Expand::Both)
    }
}

/// How two marks of the same kind and value behave when they overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Overlap {
    /// Union into one mark (bold + bold).
    Merge,
    /// Both coexist, each keeps its identity (two links, two names).
    Multiple,
}

/// Behaviour of one mark kind: expansion, overlap and point rules.
pub trait MarkBehavior: Sync {
    /// Stable name (`strong`, `link`, …).
    fn name(&self) -> &'static str;
    /// Boundary behaviour for typed text.
    fn expand(&self) -> Expand;
    /// Overlap behaviour for equal marks.
    fn overlap(&self) -> Overlap;
    /// May the mark be a point (`start == end`)?
    fn allows_point(&self) -> bool;
    /// Layer-only kinds are not part of plain Markdown files (ADR 0042 §2).
    fn is_layer(&self) -> bool;
}
