//! `fs-text`: text + layers core of the FreeSynergy wiki (ADR 0033, 0034, 0042).
//!
//! Pattern map: see `DESIGN.md`. Entry points: [`Document`] (model), [`Op`] / [`History`]
//! (commands), [`import_markdown`] / [`export_markdown`] (Markdown adapter behind the
//! [`Importer`] / [`Exporter`] traits).
#![deny(missing_docs)]

pub mod anchor;
pub mod doc;
pub mod external;
pub mod format;
pub mod history;
pub mod id;
pub mod links;
pub mod mark;
pub mod markdown;
pub mod ops;
pub mod template;
pub mod text;

pub use anchor::{Anchor, Bias};
pub use doc::{Align, Block, BlockAttrs, BlockKind, Container, Document};
pub use external::{
    diff_edits, import_external, plan_external_import, plan_external_import_with, TextEdit,
};
pub use format::{Exporter, Importer};
pub use history::{History, Retention, Step};
pub use links::{
    backlinks, extract_links, moved_path, rewrite_links, rewrite_links_with, DocOps, LinkRewriter,
    LinkRow, MovePrefix,
};
pub use mark::{
    Expand, KindSpec, LinkStyle, LinkValue, Mark, MarkBehavior, MarkKind, MarkSet, MarkValue,
    Overlap, Registry, REGISTRY,
};
pub use markdown::{
    export_markdown, import_markdown, ExportOptions, MarkWriter, MarkdownExporter,
    MarkdownImporter, MdSyntax,
};
pub use ops::{ApplyError, Command, Op};
pub use template::{instantiate, instantiate_str, DateTime, TemplateError, Values};
pub use text::{RopeStorage, Text, TextStorage};
