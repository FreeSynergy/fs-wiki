//! The port through which the wiki removes an object from the activity log.
//!
//! Pattern: Port (hexagonal) / Adapter — the core never purges a log itself: `fs-objects` says a purge
//! before the sync loses the hub's mark (ADR 0029), so a program purges the way the hub allows. What
//! stands behind the port is the program's business (the server uses `HubSession::sync_then_purge` of
//! `fs-hub-client`); the core knows only this trait, so it depends on no hub. A port that refuses
//! purges nothing (fail closed).

use async_trait::async_trait;
use fs_objects::ObjectId;

/// What a purge reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurgeOutcome {
    /// The database file was rewritten as well, so the removed bytes are gone from disk.
    pub file_cleaned: bool,
}

/// Why a port did not purge. Nothing was purged in any of these cases.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// There is no hub to sync with, and a purge must follow a sync.
    #[error("no hub is listening: a purge must follow a sync with the hub")]
    NoHub,
    /// Entries of the object are not at the hub (held back, or written while syncing).
    #[error("{0} has log entries the hub does not have yet")]
    NotSynced(ObjectId),
    /// The sync or the purge failed for another reason.
    #[error("purge refused: {0}")]
    Failed(String),
}

/// Remove an object from the activity log for good.
#[async_trait]
pub trait LogPurge: Send + Sync {
    /// Remove every log entry of `id`. `Err` means nothing was removed.
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal>;
    /// Repeat the file clean-up after a purge that could not do it; true when done.
    async fn clean_files(&self) -> Result<bool, Refusal>;
}
