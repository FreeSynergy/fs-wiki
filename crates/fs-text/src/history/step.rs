//! One undoable step.
//!
//! Pattern: Command (macro) — a step is a list of ops plus their inverses.

use crate::id::Uuid;
use crate::ops::Op;
use serde::{Deserialize, Serialize};

/// One undoable step (a group of ops, e.g. one typing burst or one formatting action).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Identity.
    pub id: Uuid,
    /// Unix seconds.
    pub at: u64,
    /// Human label.
    pub label: String,
    /// Forward ops, in application order.
    pub forward: Vec<Op>,
    /// Inverse ops, in the order they must be applied to undo (already reversed).
    pub inverse: Vec<Op>,
}
