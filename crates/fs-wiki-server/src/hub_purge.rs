//! The wiki's port for purging a log: through the hub.
//!
//! Pattern: Adapter (of the core port `LogPurge` to `fs-hub-client`) — `fs-objects` says a purge before
//! the sync loses what the hub must learn (ADR 0029), so the wiki never calls `purge_object` itself:
//! `HubSession::sync_then_purge` syncs with the hub, checks that no entry of the object is left
//! unsynced, and only then purges. No hub, a failed sync or entries the hub holds back: nothing is
//! purged (fail closed). The hub is looked for at every purge, so one started later is found; a dead
//! cached session is dropped and tried once anew; connecting and each purge have a time limit. The sync
//! and the purge run in a spawned task: a timeout or a dropped caller stops the waiting, never the work
//! (cancel safety of `fs-objects` is not relied on, see DESIGN.md "Cancelling").

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fs_hub_client::{connect, connect_at, Error as Hub, HubSession, HubStatus, Manifest};
use fs_objects::{ObjectId, Store};
use fs_wiki_core::{LogPurge, PurgeOutcome, Refusal};
use tokio::sync::Mutex;
use tokio::time::timeout;

/// The program name the wiki registers with.
pub const PROGRAM: &str = "wiki";

/// How long finding the hub and registering may take. The purge holds the wiki's write lock, so a hub
/// that accepts and then says nothing must not hold it for good.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the sync and the purge of one object may take.
pub const PURGE_TIMEOUT: Duration = Duration::from_secs(120);

/// What the wiki tells the hub when it registers: it reads nothing and writes **nothing**. The hub
/// acknowledges what the wiki has and keeps none of it, so a purge never copies a text, an old
/// revision or an image of either to the hub (a purge that first had to send the secret there would
/// defeat itself). `sync_then_purge` still passes its check: every entry is withheld and acknowledged.
#[must_use]
pub fn manifest() -> Manifest {
    Manifest::new(PROGRAM, env!("CARGO_PKG_VERSION"))
        .read(&[])
        .write(&[])
}

/// Purges through the hub of this user.
pub struct HubPurge {
    store: Arc<dyn Store>,
    socket: Option<PathBuf>,
    session: Mutex<Option<HubSession>>,
    limits: (Duration, Duration),
}

impl HubPurge {
    /// Purge in `store`, looking for the hub at `socket` (none: the default socket).
    pub fn new(store: Arc<dyn Store>, socket: Option<PathBuf>) -> Self {
        Self {
            store,
            socket,
            session: Mutex::new(None),
            limits: (CONNECT_TIMEOUT, PURGE_TIMEOUT),
        }
    }

    /// Other time limits than [`CONNECT_TIMEOUT`] and [`PURGE_TIMEOUT`] (for a test with a hub that hangs).
    #[must_use]
    pub fn with_limits(mut self, connect: Duration, purge: Duration) -> Self {
        self.limits = (connect, purge);
        self
    }

    /// The registered session, connecting if there is none yet.
    async fn session(&self) -> Result<HubSession, Refusal> {
        let mut held = self.session.lock().await;
        if let Some(s) = held.as_ref() {
            return Ok(s.clone());
        }
        let connecting = async {
            match &self.socket {
                Some(path) => connect_at(path, manifest()).await,
                None => connect(manifest()).await,
            }
        };
        let status = timeout(self.limits.0, connecting)
            .await
            .map_err(|_| Refusal::Timeout)?
            .map_err(|e| Refusal::Failed(e.to_string()))?;
        match status {
            HubStatus::Connected(s) => {
                *held = Some(s.clone());
                Ok(s)
            }
            HubStatus::Absent => Err(Refusal::NoHub),
        }
    }

    async fn forget(&self) {
        *self.session.lock().await = None;
    }
}

/// Whether `e` says the connection to the hub is gone (closed, reset, refused, disconnected): not that
/// the purge was refused, and not any other RPC or I/O failure, which stays a plain failure.
fn is_connection(e: &Hub) -> bool {
    match e {
        Hub::NoHub | Hub::Closed => true,
        Hub::Rpc(e) => e.kind == capnp::ErrorKind::Disconnected,
        Hub::Io(e) => matches!(
            e.kind(),
            ErrorKind::BrokenPipe
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::UnexpectedEof
                | ErrorKind::NotFound
                | ErrorKind::ConnectionRefused
        ),
        Hub::Stopped { cause, .. } => is_connection(cause),
        _ => false,
    }
}

fn refusal(e: Hub) -> Refusal {
    match e {
        Hub::NotSynced(id) => Refusal::NotSynced(id),
        e if is_connection(&e) => Refusal::NoHub,
        other => Refusal::Failed(other.to_string()),
    }
}

#[async_trait]
impl LogPurge for HubPurge {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        // A cached session can be dead (the hub was stopped): drop it and connect once more.
        let mut last = Refusal::NoHub;
        for _ in 0..2 {
            let session = self.session().await?;
            // Run in a task of its own: a timeout (or a caller that goes away and drops this future) stops
            // the waiting but never cuts the sync or the purge in the middle.
            let store = self.store.clone();
            let mut task = tokio::spawn(async move { session.sync_then_purge(&*store, id).await });
            let joined = timeout(self.limits.1, &mut task).await;
            let outcome = match joined {
                Ok(Ok(done)) => Ok(done),
                Ok(Err(e)) => return Err(Refusal::Failed(format!("the purge task failed: {e}"))),
                Err(elapsed) => Err(elapsed),
            };
            match outcome {
                Ok(Ok(report)) => {
                    return Ok(PurgeOutcome {
                        file_cleaned: report.file_cleaned,
                    })
                }
                Ok(Err(e)) => {
                    self.forget().await;
                    let again = is_connection(&e);
                    last = refusal(e);
                    if !again {
                        return Err(last);
                    }
                }
                Err(_) => {
                    // The task keeps running (the handle is detached); a retry purges again, which is safe.
                    self.forget().await;
                    return Err(Refusal::Timeout);
                }
            }
        }
        Err(last)
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.store
            .clean_files()
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))
    }
}
