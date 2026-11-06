//! Handlers for attribute and block-structure operations.
//!
//! Pattern: Command handler — insert and remove are each other's inverse.

use super::error::ApplyError;
use super::op::Op;
use crate::doc::{Block, BlockAttrs, Document};
use crate::id::Uuid;

pub(super) fn set_attrs(
    doc: &mut Document,
    id: Uuid,
    attrs: &BlockAttrs,
) -> Result<Op, ApplyError> {
    let b = doc.block_mut(id).ok_or(ApplyError::UnknownBlock(id))?;
    let old = std::mem::replace(&mut b.attrs, attrs.clone());
    Ok(Op::SetAttrs {
        block: id,
        attrs: old,
    })
}

pub(super) fn insert(doc: &mut Document, index: usize, block: &Block) -> Result<Op, ApplyError> {
    if index > doc.blocks.len() {
        return Err(ApplyError::BadIndex(index));
    }
    doc.blocks.insert(index, block.clone());
    Ok(Op::RemoveBlock { block: block.id })
}

pub(super) fn remove(doc: &mut Document, id: Uuid) -> Result<Op, ApplyError> {
    let i = doc.block_index(id).ok_or(ApplyError::UnknownBlock(id))?;
    let block = doc.blocks.remove(i);
    Ok(Op::InsertBlock { index: i, block })
}
