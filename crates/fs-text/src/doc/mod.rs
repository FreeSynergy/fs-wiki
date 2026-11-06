//! Document / block model (ADR 0034, 0042 §1).
//!
//! Pattern: Composite — a [`Document`] is a list of [`Block`]s; each block is text + marks +
//! attributes. A file is the degenerate case of one `Raw` block.

mod attrs;
mod block;
mod block_kind;
mod container;
mod document;
mod same;
mod source;

pub use attrs::BlockAttrs;
pub use block::Block;
pub use block_kind::{Align, BlockKind};
pub use container::Container;
pub use document::Document;
pub use source::{usable_source, Source};
