//! Retention policy of the history.
//!
//! Pattern: Strategy (data) — the humans choose how long undo steps live; `History::compact`
//! only reads this value.

use serde::{Deserialize, Serialize};

/// How long undo steps are kept. The humans choose: by age, by count, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retention {
    /// Maximum age in seconds.
    pub max_age: Option<u64>,
    /// Maximum number of undoable steps.
    pub max_steps: Option<usize>,
}
