//! Adapter from the `Store` facade to the two narrow traits the wiki needs.
//!
//! Pattern: Adapter — `WikiRepo::new` takes `Arc<dyn Store>`, the repository itself only sees
//! `dyn ObjectRepository`, `dyn ObjectQuery` and, for the removal of an object from the log, the
//! small `LogPurge`.

use std::sync::Arc;

use async_trait::async_trait;
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{
    Activity, LogEntry, ObjectId, Query, Result, Store, StoreId, StoreInfo, StoredObject,
};

/// Removing an object from the activity log for good (`purge` and `purgeRevision`): the part of
/// `HistoryRetention` the wiki needs, so a narrow backend can leave it out.
#[async_trait]
pub(super) trait LogPurge: Send + Sync {
    /// Remove every log entry of `id`; true if the file was cleaned too.
    async fn purge_log(&self, id: ObjectId) -> Result<bool>;
    /// Repeat the file clean-up after a purge that could not do it.
    async fn clean_files(&self) -> Result<bool>;
}

#[async_trait]
impl LogPurge for StoreAdapter {
    async fn purge_log(&self, id: ObjectId) -> Result<bool> {
        Ok(Store::purge_object(&*self.0, id).await?.file_cleaned)
    }

    async fn clean_files(&self) -> Result<bool> {
        Store::clean_files(&*self.0).await
    }
}

/// Presents an `Arc<dyn Store>` as the narrow traits.
pub(super) struct StoreAdapter(pub(super) Arc<dyn Store>);

#[async_trait]
impl ObjectRepository for StoreAdapter {
    async fn put(&self, object: StoredObject, actor: &str) -> Result<Activity> {
        Store::put(&*self.0, object, actor).await
    }

    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> Result<Vec<Activity>> {
        Store::put_and_log(&*self.0, object, actor, extra).await
    }

    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        Store::get(&*self.0, id).await
    }

    async fn delete(&self, id: ObjectId, actor: &str) -> Result<Activity> {
        Store::delete(&*self.0, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for StoreAdapter {
    async fn query(&self, query: &Query) -> Result<Vec<StoredObject>> {
        Store::query(&*self.0, query).await
    }

    async fn count(&self, query: &Query) -> Result<usize> {
        Store::count(&*self.0, query).await
    }
}

impl StoreInfo for StoreAdapter {
    fn identity(&self) -> StoreId {
        Store::identity(&*self.0)
    }

    fn is_encrypted(&self) -> bool {
        Store::is_encrypted(&*self.0)
    }
}
