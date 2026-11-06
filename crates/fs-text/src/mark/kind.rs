//! The closed set of mark kinds.
//!
//! Pattern: Registry key — [`MarkKind`] is just the identity; behaviour is looked up in the
//! [`crate::Registry`] through [`MarkKind::behavior`].

use super::behavior::MarkBehavior;
use super::registry::{KindSpec, Registry};
use serde::{Deserialize, Serialize};

/// What a mark means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MarkKind {
    /// Bold.
    Strong,
    /// Italic.
    Emphasis,
    /// Inline code.
    Code,
    /// Strike-through.
    Strike,
    /// Link; value = [`crate::LinkValue`]. Target is a UUID plus the readable path.
    Link,
    /// Image / embed; value = [`crate::LinkValue`] (alt text is the marked range).
    Image,
    /// Verbatim inline HTML.
    Html,
    /// Named passage (range, value = name). ADR 0034 / 0033: content that can be reused.
    Passage,
    /// Variable placeholder (point). ADR 0033: filled in on instantiation.
    Variable,
    /// Own explanation replacing / muting the original text (range).
    Explanation,
    /// Footnote reference (point, value = label).
    FootnoteRef,
}

impl MarkKind {
    /// The registered behaviour of this kind.
    #[must_use]
    pub fn behavior(self) -> &'static dyn MarkBehavior {
        Registry::get(self)
    }
    /// The registry entry (plain data form of [`MarkKind::behavior`]).
    #[must_use]
    pub fn spec(self) -> &'static KindSpec {
        Registry::spec(self)
    }
    /// Is this kind part of plain Markdown (so a file export contains it)?
    #[must_use]
    pub fn is_markdown_native(self) -> bool {
        !self.behavior().is_layer()
    }
}
