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

pub mod args;
pub mod hash;
pub mod import;
pub mod map;
pub mod report;
pub mod source;
pub mod store_path;
pub mod sync;

pub use import::import_folder;
pub use map::to_document;
pub use report::{Conflict, Report};
pub use source::export;
pub use source::{
    Failure, PROBLEMS_PATH, WikiSource, export_markdown_to, resolve, resolve_with_folders,
};
