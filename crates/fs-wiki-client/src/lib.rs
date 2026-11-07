//! `fs-wiki-client`: the wiki program as a client of its own contract `texts.capnp`.
//!
//! [`RpcTexts`] implements [`fs_wiki_core::Texts`] and [`fs_wiki_core::TextFeed`] over a Unix socket,
//! so a frontend depends on the traits and talks to a running `fs-wiki-server`, never to the store
//! behind it (ADR 0021). What a method cannot do answers the `Unsupported` of the provider rule
//! (ADR 0045); `DESIGN.md` lists the limits.
//!
//! Interface status: draft (ADR 0045), as the contract.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// A closure reads better than the full path of a generated reader type.
#![allow(
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    // The generated `ChangeListener` trait is async for every method, also those that never wait.
    clippy::unused_async_trait_impl
)]

mod calls;
mod feed;
mod feed_state;
mod peer;
mod problems;
#[cfg(test)]
mod problems_table;
mod quoted;
mod read;
mod remote;
mod texts_impl;

pub use fs_interfaces::rpc::client::{Timeouts, CALL_TIMEOUT, CONNECT_TIMEOUT};
pub use peer::RpcTexts;
