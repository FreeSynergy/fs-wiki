//! The kind registry (ADR 0033): kind -> behaviour.
//!
//! Pattern: Registry — a static table maps every [`MarkKind`] to its [`KindSpec`], which is
//! the stock [`MarkBehavior`] strategy. How a kind is *written* is the format adapter's
//! business (see `markdown::syntax`), so this table stays format-free.
//! (`heading` is a block attribute, see [`crate::BlockKind::Heading`].)

use super::behavior::{Expand, MarkBehavior, Overlap};
use super::kind::MarkKind;

/// Registry entry for a mark kind (data-driven [`MarkBehavior`]).
#[derive(Clone, Copy, Debug)]
pub struct KindSpec {
    /// The kind this entry describes.
    pub kind: MarkKind,
    /// Stable name.
    pub name: &'static str,
    /// Boundary behaviour.
    pub expand: Expand,
    /// Overlap behaviour.
    pub overlap: Overlap,
    /// May be a point (start == end)?
    pub point: bool,
    /// Layer only (not written to plain Markdown)?
    pub layer: bool,
}

impl MarkBehavior for KindSpec {
    fn name(&self) -> &'static str {
        self.name
    }
    fn expand(&self) -> Expand {
        self.expand
    }
    fn overlap(&self) -> Overlap {
        self.overlap
    }
    fn allows_point(&self) -> bool {
        self.point
    }
    fn is_layer(&self) -> bool {
        self.layer
    }
}

const fn spec(
    kind: MarkKind,
    name: &'static str,
    expand: Expand,
    overlap: Overlap,
    point: bool,
    layer: bool,
) -> KindSpec {
    KindSpec {
        kind,
        name,
        expand,
        overlap,
        point,
        layer,
    }
}

/// All registered kinds.
pub const REGISTRY: &[KindSpec] = &[
    spec(
        MarkKind::Strong,
        "strong",
        Expand::After,
        Overlap::Merge,
        false,
        false,
    ),
    spec(
        MarkKind::Emphasis,
        "emphasis",
        Expand::After,
        Overlap::Merge,
        false,
        false,
    ),
    spec(
        MarkKind::Code,
        "code",
        Expand::After,
        Overlap::Merge,
        false,
        false,
    ),
    spec(
        MarkKind::Strike,
        "strike",
        Expand::After,
        Overlap::Merge,
        false,
        false,
    ),
    spec(
        MarkKind::Link,
        "link",
        Expand::None,
        Overlap::Multiple,
        false,
        false,
    ),
    spec(
        MarkKind::Image,
        "image",
        Expand::None,
        Overlap::Multiple,
        false,
        false,
    ),
    spec(
        MarkKind::Html,
        "html",
        Expand::None,
        Overlap::Multiple,
        false,
        false,
    ),
    spec(
        MarkKind::Passage,
        "passage",
        Expand::None,
        Overlap::Multiple,
        false,
        true,
    ),
    spec(
        MarkKind::Variable,
        "variable",
        Expand::None,
        Overlap::Multiple,
        true,
        true,
    ),
    spec(
        MarkKind::Explanation,
        "explanation",
        Expand::None,
        Overlap::Multiple,
        false,
        true,
    ),
    spec(
        MarkKind::FootnoteRef,
        "footnote-ref",
        Expand::None,
        Overlap::Multiple,
        true,
        false,
    ),
];

/// Lookup facade over [`REGISTRY`].
pub struct Registry;

impl Registry {
    /// The entry for `kind`.
    ///
    /// # Panics
    /// Never for a kind of this crate: every kind is registered (tested).
    #[must_use]
    pub fn spec(kind: MarkKind) -> &'static KindSpec {
        REGISTRY
            .iter()
            .find(|s| s.kind == kind)
            .expect("every kind is registered")
    }
    /// The behaviour strategy for `kind`.
    #[must_use]
    pub fn get(kind: MarkKind) -> &'static dyn MarkBehavior {
        Self::spec(kind)
    }
    /// All entries.
    #[must_use]
    pub fn all() -> &'static [KindSpec] {
        REGISTRY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_registered_once() {
        for s in REGISTRY {
            assert_eq!(s.kind.spec().name, s.name);
            assert_eq!(REGISTRY.iter().filter(|o| o.kind == s.kind).count(), 1);
        }
        assert!(MarkKind::Variable.behavior().allows_point());
        assert!(!MarkKind::Strong.behavior().is_layer());
    }
}
