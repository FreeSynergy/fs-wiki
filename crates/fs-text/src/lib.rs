//! `fs-text`: text + layers core of the FreeSynergy wiki (ADR 0033, 0034, 0042).
pub mod anchor;
pub mod doc;
pub mod id;
pub mod links;
pub mod external;
pub mod mark;
pub mod md_export;
pub mod md_import;
pub mod ops;
pub mod template;
pub mod text;

pub use anchor::{Anchor, Bias};
pub use doc::*;
pub use mark::*;
pub use md_export::{export_markdown, ExportOptions};
pub use md_import::import_markdown;
pub use ops::*;
pub use text::Text;
pub use links::{extract_links, rewrite_links, LinkRow};
pub use template::{instantiate, instantiate_str, DateTime, Values};
pub use external::{diff_edits, import_external, plan_external_import, TextEdit};
pub use links::{backlinks, moved_path, DocOps};
pub use template::TemplateError;
