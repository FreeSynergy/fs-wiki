//! A store decorator that counts what the queries cost, so a test can compare two sizes.
//!
//! Two numbers: `rows`, the objects the queries hand back; and `touched`, which counts for a query
//! ordered by an index field every object of its type and context instead (the store computes the
//! sort key of each candidate before it cuts, whatever the conditions leave), so a lookup of one
//! text that asks for an order is seen at what it costs.
//! A fixed time says nothing on another machine; a ratio of counts does.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{Activity, LogEntry, ObjectId, Query, Result, Store, StoredObject, TursoStore};
use fs_wiki_core::WikiRepo;

use super::ME;

pub struct Counting {
    store: Arc<dyn Store>,
    pub rows: AtomicUsize,
    pub touched: AtomicUsize,
}

impl Counting {
    /// A wiki over a counting decorator of a fresh in-memory store.
    pub async fn wiki() -> (Arc<Counting>, Arc<WikiRepo>) {
        let counting = Arc::new(Counting {
            store: Arc::new(TursoStore::open_in_memory().await.unwrap()),
            rows: AtomicUsize::new(0),
            touched: AtomicUsize::new(0),
        });
        let repo = WikiRepo::with_backend(counting.clone(), counting.clone(), ME);
        (counting, Arc::new(repo))
    }

    pub fn reset(&self) {
        self.rows.store(0, Ordering::Relaxed);
        self.touched.store(0, Ordering::Relaxed);
    }

    pub fn rows(&self) -> usize {
        self.rows.load(Ordering::Relaxed)
    }

    pub fn touched(&self) -> usize {
        self.touched.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ObjectRepository for Counting {
    async fn put(&self, object: StoredObject, actor: &str) -> Result<Activity> {
        self.store.put(object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> Result<Vec<Activity>> {
        self.store.put_and_log(object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        self.store.get(id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> Result<Activity> {
        self.store.delete(id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Counting {
    async fn query(&self, query: &Query) -> Result<Vec<StoredObject>> {
        let found = self.store.query(query).await?;
        self.rows.fetch_add(found.len(), Ordering::Relaxed);
        let touched = if query.order_by_index.is_some() {
            let mut candidates = Query::new();
            candidates.object_type.clone_from(&query.object_type);
            candidates.context.clone_from(&query.context);
            self.store.count(&candidates).await?
        } else {
            found.len()
        };
        self.touched.fetch_add(touched, Ordering::Relaxed);
        Ok(found)
    }
    async fn count(&self, query: &Query) -> Result<usize> {
        self.store.count(query).await
    }
}
