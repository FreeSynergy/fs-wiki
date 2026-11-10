//! `fs-wiki-app`: the first visible part of the wiki program.
//!
//! A thin shell: imports a folder of Markdown files into the wiki (`import`), maps a text of the
//! wiki to the neutral blocks of `fs-components` (`map`), and offers the wiki to the reader
//! workbench as its data source (`source`). No Blinc type appears here (ADR 0009).
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

pub mod admin;
pub mod args;
mod check;
pub mod drafts;
mod drafts_orphans;
pub mod hash;
pub mod import;
pub mod map;
pub mod origins;
mod origins_log;
pub mod report;
pub mod server;
pub mod source;
pub mod store_path;
pub mod sync;
pub mod wiki;
pub mod words;

pub use drafts::FileDraftStore;
pub use import::{import_folder, import_folder_within};
pub use map::to_document;
pub use origins::{FileOrigins, OriginsError};
pub use report::{Conflict, Report};
pub use source::export;
pub use source::{
    Failure, PROBLEMS_PATH, WikiSource, export_markdown_to, resolve, resolve_with_folders,
};
pub use wiki::Wiki;
