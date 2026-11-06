//! Enclosing structure of a block (quote, callout, list item, footnote).
//!
//! Pattern: Composite (flattened) — nesting is a list of containers on each block, so blocks
//! stay a flat sequence that is easy to diff and edit.

use serde::{Deserialize, Serialize};

/// Enclosing structure of a block. `inst` identifies one instance (so consecutive blocks
/// of the same list item / quote can be told apart from the next item).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    /// `> quote`
    Quote {
        /// Instance number.
        inst: u32,
    },
    /// `> [!type]± title`
    Callout {
        /// Instance number.
        inst: u32,
        /// Callout type (`note`, `warning`, …).
        ty: String,
        /// Title text.
        title: String,
        /// Fold marker `+` / `-`.
        fold: Option<char>,
    },
    /// List item.
    Item {
        /// Instance number.
        inst: u32,
        /// Instance of the list it belongs to.
        list: u32,
        /// Ordinal for ordered lists.
        number: Option<u64>,
        /// Bullet / delimiter character.
        marker: char,
        /// Tight list (no blank lines between items)?
        tight: bool,
        /// Task state `[ ]` / `[x]`.
        task: Option<bool>,
    },
    /// Footnote definition.
    Footnote {
        /// Instance number.
        inst: u32,
        /// Footnote label.
        label: String,
    },
}

impl Container {
    /// Instance number.
    #[must_use]
    pub fn inst(&self) -> u32 {
        match self {
            Container::Quote { inst }
            | Container::Callout { inst, .. }
            | Container::Item { inst, .. }
            | Container::Footnote { inst, .. } => *inst,
        }
    }
    /// Renumber the instance.
    pub fn set_inst(&mut self, new: u32) {
        match self {
            Container::Quote { inst }
            | Container::Callout { inst, .. }
            | Container::Item { inst, .. }
            | Container::Footnote { inst, .. } => *inst = new,
        }
    }
}
