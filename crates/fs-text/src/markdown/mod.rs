//! Markdown adapter: import to the block model and export back (ADR 0042).
//!
//! Pattern: Adapter — [`MarkdownImporter`] / [`MarkdownExporter`] implement the format
//! traits; `import` and `export` hold the (pulldown-cmark based) machinery.

mod adapter;
mod export;
mod import;
mod syntax;

pub use adapter::{MarkdownExporter, MarkdownImporter};
pub use export::{
    export_blocks, export_blocks_report, export_markdown, export_markdown_report, ExportOptions,
    ExportReport, Inexact,
};
pub(crate) use import::import_plain;
pub use import::{convert_admonitions, import_markdown, HARD_BREAK};
pub use syntax::{md_syntax, MarkWriter, MdSyntax};
