//! What a block is (paragraph, heading, code, …).
//!
//! Pattern: State-like enum — the kind carries exactly the attributes that kind needs.

use serde::{Deserialize, Serialize};

/// Table column alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    /// Not specified.
    None,
    /// Left.
    Left,
    /// Centre.
    Center,
    /// Right.
    Right,
}

/// Kind of a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    /// Whole file text (code mode / external file). Exported verbatim.
    Raw,
    /// Paragraph.
    Paragraph,
    /// Heading is a *block attribute*, not a mark.
    Heading {
        /// Level 1..=6.
        level: u8,
    },
    /// Fenced code; `info` is the info string (language).
    Code {
        /// Info string.
        info: String,
    },
    /// Verbatim HTML block.
    Html,
    /// Horizontal rule.
    ThematicBreak,
    /// Cells separated by `'\t'`, rows by `'\n'` in the block text; first row is the header.
    Table {
        /// Column alignment.
        align: Vec<Align>,
    },
    /// YAML front matter (without the `---` fences).
    FrontMatter,
}
