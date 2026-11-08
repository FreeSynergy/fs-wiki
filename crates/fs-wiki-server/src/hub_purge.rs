//! The wiki's port for purging a log: through the hub.
//!
//! Pattern: Adapter (of the core port `LogPurge` to `fs-hub-client`) — `fs-objects` says a purge before
//! the sync loses what the hub must learn (ADR 0029), so the wiki never calls `purge_object` itself:
//! `HubSession::sync_then_purge` syncs with the hub, checks that no entry of the object is left
//! unsynced, and only then purges. No hub, a failed sync or entries the hub holds back: nothing is
//! purged (fail closed). The hub is looked for at every purge, so one started later is found.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use fs_hub_client::{connect, connect_at, Error as Hub, HubSession, HubStatus, Manifest};
use fs_objects::{ObjectId, Store};
use fs_wiki_core::{LogPurge, PurgeOutcome, Refusal};
use tokio::sync::Mutex;

/// The program name the wiki registers with.
pub const PROGRAM: &str = "wiki";

/// What the wiki tells the hub when it registers: it writes its texts and reads nothing. The records
/// that belong to a text (revisions with old documents, folders, order lists) are not listed, so the hub
/// holds them back and never has an old document that a purge could not reach.
#[must_use]
pub fn manifest() -> Manifest {
    Manifest::new(PROGRAM, env!("CARGO_PKG_VERSION"))
        .read(&[])
        .write(&["Article"])
}

/// Purges through the hub of this user.
pub struct HubPurge {
    store: Arc<dyn Store>,
    socket: Option<PathBuf>,
    session: Mutex<Option<HubSession>>,
}

impl HubPurge {
    /// Purge in `store`, looking for the hub at `socket` (none: the default socket).
    pub fn new(store: Arc<dyn Store>, socket: Option<PathBuf>) -> Self {
        Self {
            store,
            socket,
            session: Mutex::new(None),
        }
    }

    /// The registered session, connecting if there is none yet.
    async fn session(&self) -> Result<HubSession, Refusal> {
        let mut held = self.session.lock().await;
        if let Some(s) = held.as_ref() {
            return Ok(s.clone());
        }
        let status = match &self.socket {
            Some(path) => connect_at(path, manifest()).await,
            None => connect(manifest()).await,
        }
        .map_err(|e| Refusal::Failed(e.to_string()))?;
        match status {
            HubStatus::Connected(s) => {
                *held = Some(s.clone());
                Ok(s)
            }
            HubStatus::Absent => Err(Refusal::NoHub),
        }
    }
}

fn refusal(e: Hub) -> Refusal {
    match e {
        Hub::NoHub => Refusal::NoHub,
        Hub::NotSynced(id) => Refusal::NotSynced(id),
        Hub::Stopped { cause, .. } if matches!(*cause, Hub::NoHub) => Refusal::NoHub,
        other => Refusal::Failed(other.to_string()),
    }
}

#[async_trait]
impl LogPurge for HubPurge {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        let session = self.session().await?;
        match session.sync_then_purge(&*self.store, id).await {
            Ok(report) => Ok(PurgeOutcome {
                file_cleaned: report.file_cleaned,
            }),
            Err(e) => {
                // A dead connection is dropped, so the next purge connects anew.
                *self.session.lock().await = None;
                Err(refusal(e))
            }
        }
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.store
            .clean_files()
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))
    }
}
