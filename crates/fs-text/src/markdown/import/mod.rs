//! Markdown -> model (ADR 0042: text + marks).
//!
//! Pattern: Builder driven by events — `pulldown-cmark` events feed the importer state
//! (`state`), which each concern (block, container, table, inline, …) extends with its own
//! handlers; `run` is the event loop.

mod admonition;
mod block;
mod blockref;
mod callout;
mod container;
mod events;
mod frontmatter;
mod hints;
mod inline;
mod leaf;
mod loose;
mod options;
mod refdef;
mod run;
mod separator;
mod attach;
mod state;
mod table;
mod wikilink;

pub use admonition::convert_admonitions;
pub use options::HARD_BREAK;
pub use run::import_markdown;
pub(crate) use run::import_plain;
