//! Inline rendering: text with marks to Markdown.
//!
//! Pattern: Strategy (per mark syntax) inside a sweep — `plan` selects and clips the marks,
//! `emit` sweeps the text opening and closing them through `MarkWriter`, `escape` protects
//! literal text, `atomic` writes marks whose content is verbatim, `verify` reads a whole
//! block back and picks the form that reads right.

mod atomic;
mod ctx;
mod delim;
mod emit;
mod escape;
mod plan;
mod runs;
mod variant;
mod verify;
mod writer;

pub(super) use ctx::Ctx;
pub(super) use emit::sweep as inline;
pub(super) use variant::Variant;
pub(super) use verify::choose;
pub(super) use writer::literal;
