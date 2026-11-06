//! The store-backed wiki: module map.
//!
//! Pattern: Facade (declarations) — `WikiRepo` is the one entry point; the files behind it are
//! one concept each (record, name, read, write, move, links, transfer).

mod backend;
mod book;
mod handle;
mod links;
mod moving;
mod name;
mod read;
mod record;
mod transfer;
mod write;

pub use handle::{WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
