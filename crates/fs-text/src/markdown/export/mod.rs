//! Model -> Markdown. Canonical style: bullets are kept as imported, `*emphasis*`,
//! `**strong**`, fenced code, padded-header tables. Layers (names, variables, explanations)
//! are *not* written unless [`ExportOptions::layers`] is set (ADR 0042 §2).
//!
//! Pattern: Template method — `document` walks the blocks, `layout` places each block in its
//! containers, `leaf` renders the block kinds and `inline` renders marks (Strategy via
//! `MarkWriter`).

mod callout;
mod code;
mod container;
mod document;
mod inline;
mod keep;
mod layout;
mod leaf;
mod options;
mod table;

pub use document::{export_blocks, export_markdown};
pub use options::ExportOptions;
