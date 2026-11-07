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

mod errors;
mod feed;
mod links;
mod texts_service;
mod wire;

pub use texts_service::{TextsServer, CONTRACT_VERSION};
pub use wire::MAX_SOURCE_BYTES;
