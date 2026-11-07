//! The store-backed wiki: module map.
//!
//! Pattern: Facade (declarations) — `WikiRepo` is the one entry point; the files behind it are
//! one concept each (record, name, read, write, move, folder move, links, transfer).

mod aux;
mod backend;
mod book;
mod definitions;
mod feed;
mod folder;
mod handle;
mod keys;
mod links;
mod move_log;
mod moving;
mod name;
mod origin;
mod page;
mod plan;
mod read;
mod record;
mod relink;
mod shift;
mod target;
mod titles;
mod transfer;
mod upgrade;
mod write;

pub use handle::{WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
pub use origin::{FileOrigin, Origins};
pub use upgrade::IndexReport;
