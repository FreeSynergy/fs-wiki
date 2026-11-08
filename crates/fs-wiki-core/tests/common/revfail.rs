//! A store that fails or delays on command: a revision record that is refused, a delete that is
//! refused, every access to a text slowed down (so another task runs in between).

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{
    Activity, Error as StoreError, LogEntry, ObjectId, Result as StoreResult, Store, StoredObject,
    TursoStore,
};
use fs_wiki_core::WikiRepo;

use super::ME;

pub struct RevFail {
    pub inner: Arc<TursoStore>,
    /// Revision records still accepted; the one after that fails (`usize::MAX` = all).
    pub revisions_left: AtomicUsize,
    pub fail_delete: AtomicBool,
    /// Deletes still accepted; the one after that fails (`usize::MAX` = all).
    pub deletes_left: AtomicUsize,
    pub delay_ms: AtomicUsize,
}

impl RevFail {
    async fn slow(&self) {
        let d = self.delay_ms.load(Ordering::SeqCst);
        if d > 0 {
            tokio::time::sleep(Duration::from_millis(d as u64)).await;
        }
    }
}

#[async_trait]
impl ObjectRepository for RevFail {
    async fn put(&self, object: StoredObject, actor: &str) -> StoreResult<Activity> {
        let text = object.envelope.object_type == "Article";
        if object.envelope.object_type == "WikiRevision" {
            let left = self.revisions_left.load(Ordering::SeqCst);
            if left == 0 {
                return Err(StoreError::Backend("injected revision failure".into()));
            }
            if left != usize::MAX {
                self.revisions_left.store(left - 1, Ordering::SeqCst);
            }
        }
        if text {
            self.slow().await;
        }
        let done = ObjectRepository::put(&*self.inner, object, actor).await;
        if text {
            self.slow().await;
        }
        done
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> StoreResult<Vec<Activity>> {
        ObjectRepository::put_and_log(&*self.inner, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> StoreResult<Option<StoredObject>> {
        let found = ObjectRepository::get(&*self.inner, id).await?;
        if found
            .as_ref()
            .is_some_and(|o| o.envelope.object_type == "Article")
        {
            self.slow().await;
        }
        Ok(found)
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> StoreResult<Activity> {
        if self.fail_delete.swap(false, Ordering::SeqCst) {
            return Err(StoreError::Backend("injected delete failure".into()));
        }
        let left = self.deletes_left.load(Ordering::SeqCst);
        if left == 0 {
            return Err(StoreError::Backend("injected delete failure".into()));
        }
        if left != usize::MAX {
            self.deletes_left.store(left - 1, Ordering::SeqCst);
        }
        ObjectRepository::delete(&*self.inner, id, actor).await
    }
}

/// A wiki over a store that can fail; the same store is its log purge.
pub async fn make() -> (Arc<RevFail>, Arc<WikiRepo>, Arc<TursoStore>) {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let rf = Arc::new(RevFail {
        inner: store.clone(),
        revisions_left: AtomicUsize::new(usize::MAX),
        fail_delete: AtomicBool::new(false),
        deletes_left: AtomicUsize::new(usize::MAX),
        delay_ms: AtomicUsize::new(0),
    });
    let reads: Arc<dyn ObjectQuery> = store.clone();
    let as_store: Arc<dyn Store> = store.clone();
    let repo = WikiRepo::with_backend(rf.clone(), reads, ME).with_log_purge(as_store);
    (rf, Arc::new(repo), store)
}
