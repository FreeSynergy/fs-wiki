//! Purging through the hub, over a real socket and a real hub daemon: `purge` and `purgeRevision` need
//! the hub; trash, restore and the rest do not. The hub must come away holding nothing of the wiki.
//!
//! Pattern: Example-based integration tests against the real server, the real `HubPurge` port and a
//! real `fs-hub` daemon.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::v2::{p, short, trash, trash_ids};
use common::{create, set_id, with_provider, ME};
use fs_hub::{Config, Daemon};
use fs_objects::{ActivityFilter, Store, TursoStore};
use fs_wiki_core::{TextFormat, Texts, WikiRepo};
use fs_wiki_server::HubPurge;

fn port(store: &Arc<TursoStore>, socket: std::path::PathBuf) -> HubPurge {
    let as_store: Arc<dyn Store> = store.clone();
    HubPurge::new(as_store, Some(socket))
}

async fn wiki_over(socket: std::path::PathBuf) -> (Arc<WikiRepo>, Arc<TursoStore>) {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let as_store: Arc<dyn Store> = store.clone();
    let repo = WikiRepo::new(as_store, ME).with_log_purge(Arc::new(port(&store, socket)));
    (Arc::new(repo), store)
}

async fn log(store: &TursoStore) -> String {
    let all = store.activities(&ActivityFilter::default()).await.unwrap();
    serde_json::to_string(&all.iter().map(|a| &a.payload).collect::<Vec<_>>()).unwrap()
}

/// Every byte in the hub's data folder, as text.
fn hub_bytes(dir: &std::path::Path) -> String {
    let mut out = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        if let Ok(b) = std::fs::read(e.unwrap().path()) {
            out.push_str(&String::from_utf8_lossy(&b));
        }
    }
    out
}

fn doc_with(t: &fs_wiki_core::TextEntry, md: &str) -> fs_text::Document {
    let mut d = t.document.clone();
    TextFormat::Markdown.import_external(&mut d, md);
    d
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

#[tokio::test(flavor = "multi_thread")]
async fn the_hub_comes_away_with_nothing_of_the_wiki_and_a_dead_hub_is_found_again() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("run/hub.sock");
    let hubdata = dir.path().join("hub-data");
    let hub = Daemon::spawn(Config::new(socket.clone(), hubdata.clone())).unwrap();
    let (w, store) = wiki_over(socket.clone()).await;
    w.create("other", "OTHER-TEXT\n").await.unwrap();
    let t = w.create("a", "REV1-SECRET\n").await.unwrap();
    let t = w.edit(t.id, doc_with(&t, "REV2-SECRET\n")).await.unwrap();
    let t = w.edit(t.id, doc_with(&t, "REV3\n")).await.unwrap();
    let t = w.edit(t.id, doc_with(&t, "REV4\n")).await.unwrap();
    w.purge_revision(t.id, 1).await.unwrap();
    w.purge_revision(t.id, 2).await.unwrap();
    assert!(!log(&store).await.contains("REV1-SECRET"));
    let t = w.get(t.id).await.unwrap().unwrap();
    let t = w.edit(t.id, doc_with(&t, "TRASH-SECRET\n")).await.unwrap();
    w.trash(t.id).await.unwrap();
    w.purge(t.id).await.unwrap();
    let local = log(&store).await;
    assert!(!local.contains("TRASH-SECRET") && !local.contains("REV3"));
    assert!(w.list_trash(None, 10).await.unwrap().items.is_empty());
    let seen = hub_bytes(&hubdata);
    for needle in [
        "REV1-SECRET",
        "REV2-SECRET",
        "TRASH-SECRET",
        "OTHER-TEXT",
        "WikiRevision",
    ] {
        assert!(!seen.contains(needle), "the hub holds {needle}");
    }
    // The hub stops: a purge is refused as "no hub" and changes nothing; it is found again after a restart.
    drop(hub);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let b = w.create("b", "B-SECRET\n").await.unwrap();
    w.trash(b.id).await.unwrap();
    let err = w.purge(b.id).await.unwrap_err();
    assert!(
        matches!(
            err,
            fs_wiki_core::Error::PurgeRefused(fs_wiki_core::Refusal::NoHub)
        ),
        "{err:?}"
    );
    assert!(log(&store).await.contains("B-SECRET"));
    let _hub = Daemon::spawn(Config::new(socket, hubdata.clone())).unwrap();
    w.purge(b.id).await.unwrap();
    assert!(!log(&store).await.contains("B-SECRET"));
    assert!(!hub_bytes(&hubdata).contains("B-SECRET"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hub_that_accepts_and_says_nothing_does_not_hold_the_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("hang.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let held = tokio::spawn(async move {
        let mut keep = vec![];
        while let Ok((s, _)) = listener.accept().await {
            keep.push(s);
        }
    });
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let as_store: Arc<dyn Store> = store.clone();
    let hung =
        port(&store, socket).with_limits(Duration::from_millis(500), Duration::from_millis(500));
    let w = WikiRepo::new(as_store, ME).with_log_purge(Arc::new(hung));
    let t = w.create("a", "HANG-SECRET\n").await.unwrap();
    w.trash(t.id).await.unwrap();
    let started = std::time::Instant::now();
    let err = w.purge(t.id).await.unwrap_err();
    assert!(
        matches!(
            err,
            fs_wiki_core::Error::PurgeRefused(fs_wiki_core::Refusal::Timeout)
        ),
        "{err:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        log(&store).await.contains("HANG-SECRET"),
        "nothing was purged"
    );
    assert_eq!(w.list_trash(None, 10).await.unwrap().items.len(), 1);
    // The write lock is free again.
    w.create("c", "x\n").await.unwrap();
    held.abort();
}
