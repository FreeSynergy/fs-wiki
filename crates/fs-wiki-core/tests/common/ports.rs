//! Stand-ins for the port that purges a log: one that does what the hub way does after its checks, and
//! one that refuses.

#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use fs_objects::{ObjectId, Store};
use fs_wiki_core::{LogPurge, PurgeOutcome, Refusal};

/// Purges in the store itself (what `sync_then_purge` does once the hub has everything).
pub struct DirectPort(pub Arc<dyn Store>);

#[async_trait]
impl LogPurge for DirectPort {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        let report = self
            .0
            .purge_object(id)
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))?;
        Ok(PurgeOutcome {
            file_cleaned: report.file_cleaned,
        })
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.0
            .clean_files()
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))
    }
}

/// Refuses while `refuse` is set, else purges like [`DirectPort`]; counts the calls.
pub struct Gate {
    pub inner: DirectPort,
    pub refuse: std::sync::Mutex<Option<Refusal>>,
    pub calls: AtomicUsize,
}

impl Gate {
    pub fn over(store: Arc<dyn Store>) -> Arc<Self> {
        Arc::new(Self {
            inner: DirectPort(store),
            refuse: std::sync::Mutex::new(None),
            calls: AtomicUsize::new(0),
        })
    }

    pub fn set(&self, refusal: Option<Refusal>) {
        *self.refuse.lock().unwrap() = refusal;
    }
}

#[async_trait]
impl LogPurge for Gate {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(r) = self.refuse.lock().unwrap().clone() {
            return Err(r);
        }
        self.inner.purge(id).await
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.inner.clean_files().await
    }
}
