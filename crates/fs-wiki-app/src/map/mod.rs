//! From an `fs-text` document to the neutral blocks of the components.
//!
//! Pattern: Adapter — the only place that knows both models. `fs-components` has no dependency on
//! `fs-text`; what the components cannot draw arrives as plain text, never dropped.

mod blocks;
mod inline;
mod nest;
mod table;
mod title;

pub use blocks::to_document;
