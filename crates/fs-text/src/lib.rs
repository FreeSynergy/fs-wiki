//! `fs-text`: text + layers core of the FreeSynergy wiki (ADR 0033, 0034, 0042).
pub mod anchor;
pub mod doc;
pub mod id;
pub mod mark;
pub mod md_export;
pub mod md_import;
pub mod ops;
pub mod text;

pub use anchor::{Anchor, Bias};
pub use doc::*;
pub use mark::*;
pub use md_export::{export_markdown, ExportOptions};
pub use md_import::import_markdown;
pub use ops::*;
pub use text::Text;
