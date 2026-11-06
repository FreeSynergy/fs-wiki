//! Handlers for the mark operations.
//!
//! Pattern: Command handler — each handler snapshots the block's marks first; that snapshot
//! is the inverse (`SetMarks`).

use super::error::ApplyError;
use super::op::Op;
use crate::doc::{Block, Document};
use crate::id::Uuid;
use crate::mark::{Mark, MarkSet};

fn block(doc: &mut Document, id: Uuid) -> Result<&mut Block, ApplyError> {
    doc.block_mut(id).ok_or(ApplyError::UnknownBlock(id))
}

pub(super) fn set(doc: &mut Document, id: Uuid, marks: &[Mark]) -> Result<Op, ApplyError> {
    let b = block(doc, id)?;
    let before = b.marks.as_slice().to_vec();
    b.marks = MarkSet::from_vec(marks.to_vec());
    Ok(Op::SetMarks {
        block: id,
        marks: before,
    })
}

pub(super) fn add(doc: &mut Document, id: Uuid, mark: &Mark) -> Result<Op, ApplyError> {
    let b = block(doc, id)?;
    let before = b.marks.as_slice().to_vec();
    let mut m = mark.clone();
    m.end = m.end.min(b.text.len());
    m.start = m.start.min(m.end);
    b.marks.add(m);
    Ok(Op::SetMarks {
        block: id,
        marks: before,
    })
}

pub(super) fn remove(doc: &mut Document, id: Uuid, mark: Uuid) -> Result<Op, ApplyError> {
    let b = block(doc, id)?;
    let before = b.marks.as_slice().to_vec();
    b.marks.remove(mark).ok_or(ApplyError::UnknownMark(mark))?;
    Ok(Op::SetMarks {
        block: id,
        marks: before,
    })
}
