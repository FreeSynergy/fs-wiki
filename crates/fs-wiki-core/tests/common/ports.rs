//! Stand-ins for the port that purges a log: one that does what the hub way does after its checks, and
//! one that refuses.

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    /// Calls still allowed before `refuse` applies (`usize::MAX` = all).
    pub allow: AtomicUsize,
    /// Purges report that the file was not cleaned.
    pub unclean: AtomicBool,
    /// `clean_files` works.
    pub clean_ok: AtomicBool,
    pub cleans: AtomicUsize,
}

impl Gate {
    pub fn over(store: Arc<dyn Store>) -> Arc<Self> {
        Arc::new(Self {
            inner: DirectPort(store),
            refuse: std::sync::Mutex::new(None),
            calls: AtomicUsize::new(0),
            allow: AtomicUsize::new(usize::MAX),
            unclean: AtomicBool::new(false),
            clean_ok: AtomicBool::new(true),
            cleans: AtomicUsize::new(0),
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
        let left = self.allow.load(Ordering::SeqCst);
        if left == 0 {
            if let Some(r) = self.refuse.lock().unwrap().clone() {
                return Err(r);
            }
        } else if left != usize::MAX {
            self.allow.store(left - 1, Ordering::SeqCst);
        }
        if let Some(r) = self
            .refuse
            .lock()
            .unwrap()
            .clone()
            .filter(|_| left == usize::MAX)
        {
            return Err(r);
        }
        let mut done = self.inner.purge(id).await?;
        if self.unclean.load(Ordering::SeqCst) {
            done.file_cleaned = false;
        }
        Ok(done)
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.cleans.fetch_add(1, Ordering::SeqCst);
        if !self.clean_ok.load(Ordering::SeqCst) {
            return Err(Refusal::Failed("busy".into()));
        }
        self.inner.clean_files().await
    }
}
