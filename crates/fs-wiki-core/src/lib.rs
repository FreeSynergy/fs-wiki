//! `fs-wiki-core`: program core of the FreeSynergy wiki (no GUI).
//!
//! Every text is a wiki entry (ADR 0031): a [`fs_text::Document`] (text + layers) in an AS2
//! `Article` envelope, stored through `fs-objects`, which is the truth; files are exports (ADR
//! 0042). Start with the [`Texts`] trait; [`WikiRepo`] is its store-backed implementation.
//! See `DESIGN.md` for the pattern map.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Pedantic lints that would only add noise to this crate's small, documented API: every
// fallible fn returns the crate `Result` (documented once on `Error`), results of pure getters
// are used by callers anyway, and names repeat their module on purpose.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions
)]

mod definitions;
mod error;
mod feed;
mod hidden;
mod media;
mod page;
mod path;
mod reason;
mod repo;
mod texts;

pub use error::{Error, Result};
pub use feed::{Registry, Subscription, TextChange, TextFeed, TextListener};
pub use fs_interfaces::{Capabilities, Unsupported};
pub use hidden::is_hidden_block;
pub use media::{TextFormat, MARKDOWN};
pub use page::{Place, Placed, TextPage, TextQuery};
pub use path::TextPath;
pub use reason::Reason;
pub use repo::{FileOrigin, IndexReport, Origins, WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
pub use texts::{LinkEntry, Paging, TextEntry, Texts};
