//! The store-backed wiki: module map.
//!
//! Pattern: Facade (declarations) — `WikiRepo` is the one entry point; the files behind it are
//! one concept each (record, name, read, write, move, folder move, links, transfer).

mod backend;
mod book;
mod caps;
mod definitions;
mod feed;
mod flags;
mod folder;
mod handle;
mod ids;
mod keys;
mod links;
mod move_log;
mod moving;
mod name;
mod order;
mod origin;
mod page;
mod plan;
mod read;
mod record;
mod relink;
mod reordering;
mod revise;
mod revisions;
mod searching;
mod shift;
mod side_records;
mod snippets;
mod spaces;
mod target;
mod titles;
mod transfer;
mod trashing;
mod tree;
mod upgrade;
mod words;
mod write;

pub use handle::{WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
pub use origin::{FileOrigin, Foreign, Origins};
pub use upgrade::IndexReport;
