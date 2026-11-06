//! Block attributes: kind, containers and format hints.
//!
//! Pattern: Value object — plain serialisable data; the `extra` map carries source-style
//! hints (`glue`, `dashes`, `fence`, …) that exporters may use to reproduce a file.

use super::block_kind::BlockKind;
use super::container::Container;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Attributes of a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAttrs {
    /// Kind of block.
    pub kind: BlockKind,
    /// Enclosing containers, outermost first.
    pub containers: Vec<Container>,
    /// Obsidian block id (`^id`), without the caret.
    pub block_ref: Option<String>,
    /// Heading attributes `{#id .class}` (verbatim, braces included).
    pub heading_attrs: Option<String>,
    /// Free-form hints.
    pub extra: BTreeMap<String, String>,
}

impl BlockAttrs {
    /// Attributes of a plain block of `kind`.
    #[must_use]
    pub fn new(kind: BlockKind) -> Self {
        Self {
            kind,
            containers: Vec::new(),
            block_ref: None,
            heading_attrs: None,
            extra: BTreeMap::new(),
        }
    }
}
