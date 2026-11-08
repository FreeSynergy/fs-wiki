//! Purging through the hub, over a real socket and a real hub daemon: `purge` and `purgeRevision` need
//! the hub; trash, restore and the rest do not.
//!
//! Pattern: Example-based integration tests against the real server, the real `HubPurge` port and a
//! real `fs-hub` daemon.

mod common;

use std::sync::Arc;

use common::v2::{p, short, trash, trash_ids};
use common::{create, set_id, with_provider, ME};
use fs_hub::{Config, Daemon};
use fs_objects::{ActivityFilter, Store, TursoStore};
use fs_wiki_core::WikiRepo;
use fs_wiki_server::HubPurge;

async fn wiki_over(socket: std::path::PathBuf) -> (Arc<WikiRepo>, Arc<TursoStore>) {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let as_store: Arc<dyn Store> = store.clone();
    let port = Arc::new(HubPurge::new(as_store.clone(), Some(socket)));
    (
        Arc::new(WikiRepo::new(as_store, ME).with_log_purge(port)),
        store,
    )
}

async fn log(store: &TursoStore) -> String {
    let all = store.activities(&ActivityFilter::default()).await.unwrap();
    serde_json::to_string(&all.iter().map(|a| &a.payload).collect::<Vec<_>>()).unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn without_a_hub_purge_is_a_typed_problem_and_nothing_is_purged() {
    let dir = tempfile::tempdir().unwrap();
    let (repo, store) = wiki_over(dir.path().join("none/hub.sock")).await;
    with_provider(repo.clone(), repo, |root| async move {
        let a = create(&root, "a", "NOHUB-SECRET\n").await;
        trash(&root, a.id).await.unwrap();
        let mut req = root.purge_request();
        set_id(req.get().init_id(), a.id);
        let err = req.send().promise.await.err().unwrap();
        assert_eq!((short(&err).as_str(), p(&err).status), ("no-hub", 503));
        assert_eq!(trash_ids(&root, &[]).await.0, [a.id], "still in the trash");
    })
    .await;
    assert!(
        log(&store).await.contains("NOHUB-SECRET"),
        "nothing was purged"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn with_a_hub_the_wiki_syncs_first_and_then_purges() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("run/freesynergy/hub.sock");
    let _hub = Daemon::spawn(Config::new(socket.clone(), dir.path().join("hub-data"))).unwrap();
    let (repo, store) = wiki_over(socket).await;
    with_provider(repo.clone(), repo, |root| async move {
        let a = create(&root, "a", "HUB-SECRET\n").await;
        trash(&root, a.id).await.unwrap();
        let mut req = root.purge_request();
        set_id(req.get().init_id(), a.id);
        req.send().promise.await.unwrap();
        assert!(trash_ids(&root, &[]).await.0.is_empty());
    })
    .await;
    assert!(
        !log(&store).await.contains("HUB-SECRET"),
        "gone from the log"
    );
}
