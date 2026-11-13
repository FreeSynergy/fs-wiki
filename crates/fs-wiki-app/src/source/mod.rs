//! The wiki as a data source of the components.
//!
//! Pattern: Adapter — `WikiSource` implements the workbench's `DataSource` over the `Texts` trait
//! (not over `WikiRepo`). The trait is async, the workbench is not: calls go through a runtime
//! handle.

mod build;
pub mod create;
pub mod export;
mod export_fs;
mod export_page;
mod export_report;
mod ops;
mod pages;
mod problems;
mod resolve;
mod wiki;

pub use create::{CreateError, PageMaker, SavedHook};
pub use ops::{check_name, retitle};
pub use export::{
    Differs, ExportError, export_markdown_checked, export_markdown_if_seen, export_markdown_to,
    read_current,
};
pub use pages::PagesView;
pub use problems::{Failure, Notes, PROBLEMS_PATH};
pub use resolve::{resolve, resolve_with_folders};
pub use wiki::WikiSource;
