//! Operations on documents and the undo/redo log (ADR 0042 §10).
//!
//! Every [`Op`] can be applied to a [`Document`]; applying returns the exact inverse op.
//! Inverses of text edits carry a snapshot of the block's marks so undo restores marks
//! that died or were cut (wiki-ideas pitfall 8).

use crate::doc::{Block, BlockAttrs, Document};
use crate::id::{new_id, Uuid};
use crate::mark::{Mark, MarkSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    /// Replace chars `start..end` of a block by `insert`. Marks follow the normal edit
    /// rules, unless `restore` is given: then the block's marks are set to that snapshot.
    Edit { block: Uuid, start: usize, end: usize, insert: String, restore: Option<Vec<Mark>> },
    SetMarks { block: Uuid, marks: Vec<Mark> },
    AddMark { block: Uuid, mark: Mark },
    RemoveMark { block: Uuid, mark: Uuid },
    SetAttrs { block: Uuid, attrs: BlockAttrs },
    InsertBlock { index: usize, block: Block },
    RemoveBlock { block: Uuid },
}

impl Op {
    pub fn insert_text(block: Uuid, at: usize, s: &str) -> Op {
        Op::Edit { block, start: at, end: at, insert: s.to_string(), restore: None }
    }
    pub fn delete_text(block: Uuid, start: usize, end: usize) -> Op {
        Op::Edit { block, start, end, insert: String::new(), restore: None }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApplyError {
    #[error("unknown block {0}")]
    UnknownBlock(Uuid),
    #[error("unknown mark {0}")]
    UnknownMark(Uuid),
    #[error("range {0}..{1} outside block of length {2}")]
    OutOfRange(usize, usize, usize),
    #[error("block index {0} out of range")]
    BadIndex(usize),
}

impl Document {
    /// Apply an op; returns its inverse.
    pub fn apply(&mut self, op: &Op) -> Result<Op, ApplyError> {
        match op {
            Op::Edit { block, start, end, insert, restore } => {
                let b = self.block_mut(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let len = b.text.len();
                if start > end || *end > len {
                    return Err(ApplyError::OutOfRange(*start, *end, len));
                }
                let before: Vec<Mark> = b.marks.as_slice().to_vec();
                let old = b.text.slice(*start..*end);
                b.delete(*start, *end);
                b.insert(*start, insert);
                if let Some(snapshot) = restore {
                    b.marks = MarkSet::from_vec(snapshot.clone());
                }
                Ok(Op::Edit { block: *block, start: *start, end: start + insert.chars().count(), insert: old, restore: Some(before) })
            }
            Op::SetMarks { block, marks } => {
                let b = self.block_mut(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let before = b.marks.as_slice().to_vec();
                b.marks = MarkSet::from_vec(marks.clone());
                Ok(Op::SetMarks { block: *block, marks: before })
            }
            Op::AddMark { block, mark } => {
                let b = self.block_mut(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let before = b.marks.as_slice().to_vec();
                let mut m = mark.clone();
                m.end = m.end.min(b.text.len());
                m.start = m.start.min(m.end);
                b.marks.add(m);
                Ok(Op::SetMarks { block: *block, marks: before })
            }
            Op::RemoveMark { block, mark } => {
                let b = self.block_mut(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let before = b.marks.as_slice().to_vec();
                b.marks.remove(*mark).ok_or(ApplyError::UnknownMark(*mark))?;
                Ok(Op::SetMarks { block: *block, marks: before })
            }
            Op::SetAttrs { block, attrs } => {
                let b = self.block_mut(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let old = std::mem::replace(&mut b.attrs, attrs.clone());
                Ok(Op::SetAttrs { block: *block, attrs: old })
            }
            Op::InsertBlock { index, block } => {
                if *index > self.blocks.len() {
                    return Err(ApplyError::BadIndex(*index));
                }
                self.blocks.insert(*index, block.clone());
                Ok(Op::RemoveBlock { block: block.id })
            }
            Op::RemoveBlock { block } => {
                let i = self.block_index(*block).ok_or(ApplyError::UnknownBlock(*block))?;
                let b = self.blocks.remove(i);
                Ok(Op::InsertBlock { index: i, block: b })
            }
        }
    }
}

/// How long undo steps are kept. The humans choose: by age, by count, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retention {
    /// Maximum age in seconds.
    pub max_age: Option<u64>,
    pub max_steps: Option<usize>,
}

/// One undoable step (a group of ops, e.g. one typing burst or one formatting action).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: Uuid,
    /// Unix seconds.
    pub at: u64,
    pub label: String,
    /// Forward ops, in application order.
    pub forward: Vec<Op>,
    /// Inverse ops, in the order they must be applied to undo (already reversed).
    pub inverse: Vec<Op>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub retention: Retention,
    steps: Vec<Step>,
    /// Number of steps currently applied; `steps[cursor..]` can be redone.
    cursor: usize,
    /// Forward ops of compacted-away steps that the hub has not seen yet (kept until sync).
    pending_sync: Vec<Op>,
    /// Steps up to this count are known to the hub.
    synced_upto: usize,
}

impl History {
    pub fn new(retention: Retention) -> Self {
        Self { retention, ..Default::default() }
    }
    pub fn len(&self) -> usize {
        self.steps.len()
    }
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor < self.steps.len()
    }
    /// Rough size indicator for the "history size" display (number of ops kept).
    pub fn op_count(&self) -> usize {
        self.steps.iter().map(|s| s.forward.len() + s.inverse.len()).sum::<usize>() + self.pending_sync.len()
    }
    pub fn pending_sync(&self) -> &[Op] {
        &self.pending_sync
    }

    /// Apply `ops` to `doc` as one step and record it. On error the document is rolled back.
    pub fn apply(&mut self, doc: &mut Document, ops: Vec<Op>, label: &str, now: u64) -> Result<Uuid, ApplyError> {
        let mut inverse = Vec::with_capacity(ops.len());
        for (i, op) in ops.iter().enumerate() {
            match doc.apply(op) {
                Ok(inv) => inverse.push(inv),
                Err(e) => {
                    for inv in inverse.iter().rev() {
                        let _ = doc.apply(inv);
                    }
                    let _ = i;
                    return Err(e);
                }
            }
        }
        inverse.reverse();
        let id = new_id();
        self.steps.truncate(self.cursor);
        self.synced_upto = self.synced_upto.min(self.cursor);
        self.steps.push(Step { id, at: now, label: label.to_string(), forward: ops, inverse });
        self.cursor = self.steps.len();
        self.compact(now);
        Ok(id)
    }

    pub fn undo(&mut self, doc: &mut Document) -> Result<bool, ApplyError> {
        if self.cursor == 0 {
            return Ok(false);
        }
        let step = self.steps[self.cursor - 1].clone();
        for op in &step.inverse {
            doc.apply(op)?;
        }
        self.cursor -= 1;
        Ok(true)
    }

    pub fn redo(&mut self, doc: &mut Document) -> Result<bool, ApplyError> {
        if self.cursor >= self.steps.len() {
            return Ok(false);
        }
        let step = self.steps[self.cursor].clone();
        for op in &step.forward {
            doc.apply(op)?;
        }
        self.cursor += 1;
        Ok(true)
    }

    /// Drop undo steps beyond the retention. The document keeps its current state; only
    /// the intermediate steps fall away. Forward ops the hub has not seen yet move to
    /// `pending_sync` and stay until [`History::mark_synced`].
    pub fn compact(&mut self, now: u64) {
        let mut drop_n = 0;
        if let Some(max) = self.retention.max_steps {
            drop_n = drop_n.max(self.cursor.saturating_sub(max));
        }
        if let Some(age) = self.retention.max_age {
            let old = self.steps[..self.cursor].iter().take_while(|s| now.saturating_sub(s.at) > age).count();
            drop_n = drop_n.max(old);
        }
        if drop_n == 0 {
            return;
        }
        for (i, s) in self.steps.drain(..drop_n).enumerate() {
            if i >= self.synced_upto {
                self.pending_sync.extend(s.forward);
            }
        }
        self.cursor -= drop_n;
        self.synced_upto = self.synced_upto.saturating_sub(drop_n);
    }

    /// The hub has everything up to now: forget pending ops and note that all steps are synced.
    pub fn mark_synced(&mut self) {
        self.pending_sync.clear();
        self.synced_upto = self.steps.len();
    }

    /// "Reset the undo history" (offered when saving): keeps nothing undoable.
    pub fn reset(&mut self) {
        for (i, s) in self.steps.drain(..).enumerate() {
            if i >= self.synced_upto {
                self.pending_sync.extend(s.forward);
            }
        }
        self.cursor = 0;
        self.synced_upto = 0;
    }
}
