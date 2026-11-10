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
pub mod describe;
mod entry;
mod error;
mod feed;
mod hidden;
mod history;
mod limits;
mod media;
mod nodes;
mod page;
mod path;
mod purge;
mod reason;
mod redirect;
mod repo;
mod search;
mod texts;
mod trash;

pub use error::{Error, Result};
pub use feed::{Registry, Subscription, TextChange, TextFeed, TextListener};
pub use fs_interfaces::{Capabilities, Unsupported};
pub use hidden::is_hidden_block;
pub use history::{Expect, RevisionContent, RevisionInfo, RevisionPage};
pub use limits::*;
pub use media::{TextFormat, MARKDOWN};
pub use nodes::{Node, NodeKind, NodePage, Placement};
pub use page::{Place, Placed, TextPage, TextQuery};
pub use path::TextPath;
pub use purge::{LogPurge, PurgeOutcome, Refusal};
pub use reason::Reason;
pub use redirect::{Outcome, Resolution, MAX_REDIRECT_HOPS};
pub use repo::{
    FileOrigin, Foreign, IndexReport, Origins, WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE,
};
pub use search::{
    Highlight, SearchFilter, SearchHit, SearchPage, SearchPlace, SearchQuery, Snippet, Tri,
};
pub use texts::{LinkEntry, Paging, TextEntry, Texts};
pub use trash::{TrashItem, TrashPage, TrashPlace};
