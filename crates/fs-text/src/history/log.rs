//! The history log: apply with grouping, undo, redo, compaction.
//!
//! Pattern: Command invoker (Caretaker) — owns the steps and the cursor; every document
//! change goes through [`Command::apply`] so the inverse is always known.

use super::retention::Retention;
use super::step::Step;
use crate::doc::Document;
use crate::id::{new_id, Uuid};
use crate::ops::{ApplyError, Command, Op};
use serde::{Deserialize, Serialize};

/// Undo/redo log of one document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    /// Retention policy.
    pub retention: Retention,
    pub(super) steps: Vec<Step>,
    /// Number of steps currently applied; `steps[cursor..]` can be redone.
    pub(super) cursor: usize,
    /// Forward ops of compacted-away steps that the hub has not seen yet (kept until sync).
    pub(super) pending_sync: Vec<Op>,
    /// Steps up to this count are known to the hub.
    pub(super) synced_upto: usize,
}

impl History {
    /// Empty history with the given retention.
    #[must_use]
    pub fn new(retention: Retention) -> Self {
        Self {
            retention,
            ..Default::default()
        }
    }
    /// Number of steps kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }
    /// No steps kept?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
    /// Is there a step to undo?
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    /// Is there a step to redo?
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.cursor < self.steps.len()
    }
    #[must_use]
    /// Rough size indicator for the "history size" display (number of ops kept).
    pub fn op_count(&self) -> usize {
        self.steps
            .iter()
            .map(|s| s.forward.len() + s.inverse.len())
            .sum::<usize>()
            + self.pending_sync.len()
    }
    /// Forward ops of dropped steps the hub has not seen yet.
    #[must_use]
    pub fn pending_sync(&self) -> &[Op] {
        &self.pending_sync
    }

    /// Apply `ops` to `doc` as one step and record it. On error the document is rolled back.
    ///
    /// # Errors
    /// The first [`ApplyError`] of the ops.
    pub fn apply(
        &mut self,
        doc: &mut Document,
        ops: Vec<Op>,
        label: &str,
        now: u64,
    ) -> Result<Uuid, ApplyError> {
        let mut inverse = Vec::with_capacity(ops.len());
        for op in &ops {
            match op.apply(doc) {
                Ok(inv) => inverse.push(inv),
                Err(e) => {
                    for inv in inverse.iter().rev() {
                        let _ = inv.apply(doc);
                    }
                    return Err(e);
                }
            }
        }
        inverse.reverse();
        let id = new_id();
        self.steps.truncate(self.cursor);
        self.synced_upto = self.synced_upto.min(self.cursor);
        self.steps.push(Step {
            id,
            at: now,
            label: label.to_string(),
            forward: ops,
            inverse,
        });
        self.cursor = self.steps.len();
        self.compact(now);
        Ok(id)
    }

    /// Undo the last step; `false` if there is none.
    ///
    /// # Errors
    /// [`ApplyError`] if an inverse op no longer fits the document.
    pub fn undo(&mut self, doc: &mut Document) -> Result<bool, ApplyError> {
        if self.cursor == 0 {
            return Ok(false);
        }
        let step = self.steps[self.cursor - 1].clone();
        for op in &step.inverse {
            op.apply(doc)?;
        }
        self.cursor -= 1;
        Ok(true)
    }

    /// Redo the next step; `false` if there is none.
    ///
    /// # Errors
    /// [`ApplyError`] if a forward op no longer fits the document.
    pub fn redo(&mut self, doc: &mut Document) -> Result<bool, ApplyError> {
        if self.cursor >= self.steps.len() {
            return Ok(false);
        }
        let step = self.steps[self.cursor].clone();
        for op in &step.forward {
            op.apply(doc)?;
        }
        self.cursor += 1;
        Ok(true)
    }
}
