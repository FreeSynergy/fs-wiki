//! `fs-wiki-testserver`: the real `fs-wiki-server` for tests, on a real Unix socket over a real store,
//! on a thread of its own. Nothing here stands in for the server.
//!
//! Pattern: Facade — this file only declares the module and re-exports the guard.

#![deny(missing_docs)]

mod served;

pub use served::Served;
