//! The wiki's port for purging a log where there is no hub: every purge is refused as "no hub".
//!
//! Pattern: Null Object (of the core port `LogPurge`, the other side of the Strategy chosen by system in `lib.rs`) — the
//! hub (`fs-hub`) does not run on this system yet, so the answer is the one a missing hub gives anywhere: nothing is purged
//! (fail closed), the person is told `no-hub`, and the clean-up of files still works. The hub's own port to Windows lifts
//! this; the wiki changes nothing then but its dependency.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fs_objects::{ObjectId, Store};
use fs_wiki_core::{LogPurge, PurgeOutcome, Refusal};

/// The program name the wiki registers with.
pub const PROGRAM: &str = "wiki";

/// Purges nothing: no hub on this system.
pub struct HubPurge {
    store: Arc<dyn Store>,
}

impl HubPurge {
    /// As the port with a hub; the socket is not looked for.
    pub fn new(store: Arc<dyn Store>, _socket: Option<PathBuf>) -> Self {
        Self { store }
    }

    /// Time limits mean nothing without a hub.
    #[must_use]
    pub fn with_limits(self, _connect: Duration, _purge: Duration) -> Self {
        self
    }
}

#[async_trait]
impl LogPurge for HubPurge {
    async fn purge(&self, _id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        Err(Refusal::NoHub)
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.store
            .clean_files()
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))
    }
}
