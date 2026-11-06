//! The wiki as a data source of the components.
//!
//! Pattern: Adapter — `WikiSource` implements the workbench's `DataSource` over the `Texts` trait
//! (not over `WikiRepo`). The trait is async, the workbench is not: calls go through a runtime
//! handle.

mod export;
mod problems;
mod resolve;
mod wiki;

pub use export::export_markdown_to;
pub use problems::{Failure, Notes, PROBLEMS_PATH};
pub use resolve::resolve;
pub use wiki::WikiSource;
