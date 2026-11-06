//! The record of the last folder move.
//!
//! Pattern: Memento — written before the first write of a folder move and marked done after the
//! last one. It lets the program tell three things apart that the texts alone cannot: repeating
//! an interrupted move (finish it), repeating a completed move (nothing to do), and starting a
//! different move while one is unfinished (refused, or the folder would split).

use fs_store::ObjectId;
use serde::{Deserialize, Serialize};

use super::handle::WikiRepo;
use crate::error::Result;

const KIND: &str = "MoveRecord";
const KEY: &str = "last";

/// The last folder move.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MoveRecord {
    pub from: String,
    pub to: String,
    pub done: bool,
    /// Every text of the move; the target may hold these and nothing else.
    pub ids: Vec<ObjectId>,
}

impl MoveRecord {
    /// Whether this record is the move `from` -> `to`.
    pub(super) fn is(&self, from: &str, to: &str) -> bool {
        self.from == from && self.to == to
    }
}

impl WikiRepo {
    pub(super) async fn last_move(&self) -> Result<Option<MoveRecord>> {
        Ok(self.aux_get(KIND, KEY).await?.map(|(_, r)| r))
    }

    pub(super) async fn record_move(&self, record: &MoveRecord) -> Result<()> {
        self.aux_put(KIND, KEY, record).await
    }
}
