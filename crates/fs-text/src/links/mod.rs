//! Link table and link rewriting (ADR 0042 §8).
//!
//! Pattern: Strategy (rewriter) over a table — [`LinkRewriter`] decides how a path changes,
//! `rewrite` applies it to documents as undoable ops, `row` reads the table.

mod rewrite;
mod rewriter;
mod row;

pub use rewrite::{rewrite_links, rewrite_links_with, DocOps};
pub use rewriter::{moved_path, LinkRewriter, MovePrefix};
pub use row::{backlinks, extract_links, LinkRow};
