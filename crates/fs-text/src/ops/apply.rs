//! Dispatch of [`Op`] to its handlers.
//!
//! Pattern: Command (invoker) — `Op::apply` routes each variant to a small handler module
//! (`edit`, `marks`, `blocks`); `Document::apply` is the convenient entry point.

use super::command::Command;
use super::error::ApplyError;
use super::op::Op;
use super::{blocks, edit, marks};
use crate::doc::Document;

impl Command for Op {
    type Target = Document;

    fn apply(&self, doc: &mut Document) -> Result<Op, ApplyError> {
        match self {
            Op::Edit {
                block,
                start,
                end,
                insert,
                restore,
            } => edit::apply(doc, *block, *start, *end, insert, restore.as_deref()),
            Op::SetMarks { block, marks } => marks::set(doc, *block, marks),
            Op::AddMark { block, mark } => marks::add(doc, *block, mark),
            Op::RemoveMark { block, mark } => marks::remove(doc, *block, *mark),
            Op::SetAttrs { block, attrs } => blocks::set_attrs(doc, *block, attrs),
            Op::InsertBlock { index, block } => blocks::insert(doc, *index, block),
            Op::RemoveBlock { block } => blocks::remove(doc, *block),
        }
    }
}

impl Document {
    /// Apply an op; returns its inverse.
    ///
    /// # Errors
    /// [`ApplyError`] if the op does not fit the document.
    pub fn apply(&mut self, op: &Op) -> Result<Op, ApplyError> {
        Command::apply(op, self)
    }
}
