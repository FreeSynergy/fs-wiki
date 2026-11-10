//! `fs-wiki-server`: the contract `texts.capnp` over a Unix socket, in front of any provider of
//! `Texts` and `TextFeed`. It only translates: no text logic lives here.
//!
//! Interface status: draft (ADR 0045).
//!
//! Pattern: Facade — this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The generated `Server` trait is async for every method, also those that never wait.
#![allow(clippy::missing_errors_doc, clippy::unused_async_trait_impl)]

mod cursor;
mod cursors;
mod errors;
mod errors_v2;
mod facts;
mod feed;
#[cfg(unix)]
mod hub_purge;
#[cfg(not(unix))]
#[path = "hub_purge_absent.rs"]
mod hub_purge;
pub mod lease;
mod links;
mod paging;
mod program;
mod reads;
mod reads_v2;
mod texts_service;
mod wire;
mod wire_history;
mod wire_nodes;
mod wire_search;
mod writes;
mod writes_v2;

#[cfg(unix)]
pub use hub_purge::manifest;
pub use hub_purge::{HubPurge, PROGRAM};
pub use program::{serve, ServeError, HELP};
pub use texts_service::{TextsServer, CONTRACT_VERSION};
pub use wire::MAX_SOURCE_BYTES;
