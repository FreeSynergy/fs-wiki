//! Handler for [`crate::Op::Edit`].
//!
//! Pattern: Command handler — applies a text edit and builds its inverse from a snapshot of
//! the block's marks, so undo restores marks that died or were cut (wiki-ideas pitfall 8).

use super::error::ApplyError;
use super::op::Op;
use crate::doc::Document;
use crate::id::Uuid;
use crate::mark::{Mark, MarkSet};

pub(super) fn apply(
    doc: &mut Document,
    block: Uuid,
    start: usize,
    end: usize,
    insert: &str,
    restore: Option<&[Mark]>,
) -> Result<Op, ApplyError> {
    let b = doc
        .block_mut(block)
        .ok_or(ApplyError::UnknownBlock(block))?;
    let len = b.text.len();
    if start > end || end > len {
        return Err(ApplyError::OutOfRange(start, end, len));
    }
    let before: Vec<Mark> = b.marks.as_slice().to_vec();
    let old = b.text.slice(start..end);
    b.delete(start, end);
    b.insert(start, insert);
    if let Some(snapshot) = restore {
        b.marks = MarkSet::from_vec(snapshot.to_vec());
    }
    Ok(Op::Edit {
        block,
        start,
        end: start + insert.chars().count(),
        insert: old,
        restore: Some(before),
    })
}
