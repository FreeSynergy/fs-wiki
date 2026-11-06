//! Compaction, hub sync bookkeeping and reset.
//!
//! Pattern: Strategy client — [`Retention`](super::Retention) decides what is dropped;
//! dropped forward ops the hub has not seen stay in `pending_sync` (never lost).

use super::log::History;

impl History {
    /// Drop undo steps beyond the retention. The document keeps its current state; only
    /// the intermediate steps fall away. Forward ops the hub has not seen yet move to
    /// `pending_sync` and stay until [`History::mark_synced`].
    pub fn compact(&mut self, now: u64) {
        let mut drop_n = 0;
        if let Some(max) = self.retention.max_steps {
            drop_n = drop_n.max(self.cursor.saturating_sub(max));
        }
        if let Some(age) = self.retention.max_age {
            let old = self.steps[..self.cursor]
                .iter()
                .take_while(|s| now.saturating_sub(s.at) > age)
                .count();
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
