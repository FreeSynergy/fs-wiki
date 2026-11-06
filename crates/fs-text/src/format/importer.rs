//! The import interface.
//!
//! Pattern: Adapter (target interface) — foreign text in, our block document out.

use crate::doc::Document;

/// Reads a foreign format into the document model.
pub trait Importer {
    /// Parse `src` into a block document.
    fn import(&self, src: &str) -> Document;
}
