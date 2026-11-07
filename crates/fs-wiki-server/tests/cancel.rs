//! A client that goes away cannot leave a write half done.
//!
//! Pattern: Example-based integration test with a store whose writes stop at a gate.

mod common;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use common::{list_paths, with_provider, ME};
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{Activity, LogEntry, ObjectId, Query, Result, Store, StoredObject, TursoStore};
use fs_wiki_core::{Texts, WikiRepo};
use tokio::sync::{watch, Notify};

/// A store whose writes wait while the gate is closed.
struct Gated {
    store: Arc<dyn Store>,
    reached: Notify,
    open: watch::Sender<bool>,
}

impl Gated {
    async fn pass(&self) {
        self.reached.notify_one();
        let mut open = self.open.subscribe();
        let _ = open.wait_for(|open| *open).await;
    }
}

#[async_trait]
impl ObjectRepository for Gated {
    async fn put(&self, object: StoredObject, actor: &str) -> Result<Activity> {
        self.pass().await;
        self.store.put(object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> Result<Vec<Activity>> {
        self.pass().await;
        self.store.put_and_log(object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        self.store.get(id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> Result<Activity> {
        self.pass().await;
        self.store.delete(id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Gated {
    async fn query(&self, query: &Query) -> Result<Vec<StoredObject>> {
        self.store.query(query).await
    }
    async fn count(&self, query: &Query) -> Result<usize> {
        self.store.count(query).await
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_folder_move_dropped_while_it_waits_at_a_write_still_finishes() {
    let gated = Arc::new(Gated {
        store: Arc::new(TursoStore::open_in_memory().await.unwrap()),
        reached: Notify::new(),
        open: watch::channel(true).0,
    });
    let repo = Arc::new(WikiRepo::with_backend(gated.clone(), gated.clone(), ME));
    for (path, md) in [("m/a", "[[m/b]]\n"), ("m/b", "x\n"), ("m/c", "y\n")] {
        repo.create(path, md).await.unwrap();
    }
    with_provider(repo.clone(), repo, |root| async move {
        gated.open.send_replace(false);
        let mut req = root.move_folder_request();
        req.get().set_prefix("m");
        req.get().set_path("n");
        let call = req.send().promise;
        let reached = gated.reached.notified();
        tokio::time::timeout(Duration::from_secs(5), async {
            tokio::select! {
                () = reached => {}
                _ = call => panic!("the move answered before its first write"),
            }
        })
        .await
        .expect("the move reached no write");
        // The call is gone now; give the server the time to hear of it, as it stands at the gate.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(list_paths(&root, "m").await.len(), 3, "the gate holds");

        gated.open.send_replace(true);
        for _ in 0..100 {
            if list_paths(&root, "n").await.len() == 3 {
                assert!(list_paths(&root, "m").await.is_empty());
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!(
            "the move stopped halfway: {:?}",
            list_paths(&root, "").await
        );
    })
    .await;
}
