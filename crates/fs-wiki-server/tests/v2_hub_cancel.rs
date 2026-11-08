//! A purge that times out or is dropped does not cut the work: against the real hub daemon and a store in
//! a file, the sync and purge run on, a retry completes, and neither the hub's folder nor the wiki's file
//! holds the secret afterwards.
//!
//! Pattern: Example-based integration tests with tiny time limits and dropped futures.

use std::sync::Arc;
use std::time::Duration;

use fs_hub::{Config, Daemon};
use fs_objects::{ActivityFilter, Store, TursoStore};
use fs_wiki_core::{Error, Refusal, Texts, WikiRepo};
use fs_wiki_server::HubPurge;

const ME: &str = "https://example.org/actors/a";

fn bytes(dir: &std::path::Path) -> String {
    let mut out = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            out.push_str(&String::from_utf8_lossy(&std::fs::read(&p).unwrap()));
        }
    }
    out
}

async fn in_log(store: &dyn Store, needle: &str) -> bool {
    let all = store.activities(&ActivityFilter::default()).await.unwrap();
    serde_json::to_string(&all.iter().map(|a| &a.payload).collect::<Vec<_>>())
        .unwrap()
        .contains(needle)
}

struct Fixture {
    w: Arc<WikiRepo>,
    store: Arc<dyn Store>,
    wiki_dir: std::path::PathBuf,
    hub_dir: std::path::PathBuf,
    socket: std::path::PathBuf,
    _hub: Daemon,
    _tmp: tempfile::TempDir,
}

async fn fixture(purge_limit: Duration) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let wiki_dir = tmp.path().join("wiki");
    let hub_dir = tmp.path().join("hub-data");
    std::fs::create_dir_all(&wiki_dir).unwrap();
    let socket = tmp.path().join("run/hub.sock");
    let hub = Daemon::spawn(Config::new(socket.clone(), hub_dir.clone())).unwrap();
    let store: Arc<dyn Store> =
        Arc::new(TursoStore::open(wiki_dir.join("objects.db")).await.unwrap());
    let port = HubPurge::new(store.clone(), Some(socket.clone()))
        .with_limits(Duration::from_secs(30), purge_limit);
    let w = Arc::new(WikiRepo::new(store.clone(), ME).with_log_purge(Arc::new(port)));
    Fixture {
        w,
        store,
        wiki_dir,
        hub_dir,
        socket,
        _hub: hub,
        _tmp: tmp,
    }
}

async fn trashed(f: &Fixture, secret: &str) -> fs_objects::ObjectId {
    let t = f.w.create("a", &format!("{secret}\n")).await.unwrap();
    f.w.trash(t.id).await.unwrap();
    t.id
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timeout_stops_the_waiting_not_the_work_and_a_retry_completes() {
    let f = fixture(Duration::from_nanos(1)).await;
    let id = trashed(&f, "TIMEOUT-SECRET").await;
    let first = f.w.purge(id).await;
    assert!(
        matches!(first, Err(Error::PurgeRefused(Refusal::Timeout)) | Ok(())),
        "{first:?}"
    );
    // The detached task finishes the call it was in; the wiki stopped waiting, so the rest of the
    // sequence (more logs, the deletes) is for the retry. Let the task settle first.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        f.w.list_trash(None, 10).await.unwrap().items.len(),
        1,
        "text still in the trash"
    );
    // The store reads right and the write lock is free.
    f.w.create("later", "x\n").await.unwrap();
    // A retry with time to finish (a new program over the same store) completes.
    let port = HubPurge::new(f.store.clone(), Some(f.socket.clone()));
    let again = WikiRepo::new(f.store.clone(), ME).with_log_purge(Arc::new(port));
    let mut done = false;
    for _ in 0..40 {
        match again.purge(id).await {
            Ok(()) | Err(Error::NotFound(_)) | Err(Error::NotInTrash(_)) => {
                done = true;
                break;
            }
            Err(Error::PurgeRefused(Refusal::Timeout)) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!(done, "no retry completed in time");
    assert!(again.list_trash(None, 10).await.unwrap().items.is_empty());
    assert!(again.retry_file_cleanup().await.unwrap());
    assert!(!bytes(&f.wiki_dir).contains("TIMEOUT-SECRET"), "wiki file");
    assert!(!bytes(&f.hub_dir).contains("TIMEOUT-SECRET"), "hub folder");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_caller_that_goes_away_does_not_cut_the_purge() {
    let f = fixture(Duration::from_secs(120)).await;
    let id = trashed(&f, "DROPPED-SECRET").await;
    let w = f.w.clone();
    let call = tokio::spawn(async move { w.purge(id).await });
    // Client disconnect: the task that waits for the purge is aborted at once.
    tokio::task::yield_now().await;
    call.abort();
    let _ = call.await;
    // Whatever stage it reached, the store reads right and a retry completes.
    let mut done = false;
    for _ in 0..100 {
        match f.w.purge(id).await {
            Ok(()) | Err(Error::NotFound(_)) | Err(Error::NotInTrash(_)) => {
                done = true;
                break;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!(done);
    f.w.create("later", "x\n").await.unwrap();
    assert!(f.w.retry_file_cleanup().await.unwrap());
    assert!(!in_log(&*f.store, "DROPPED-SECRET").await);
    assert!(!bytes(&f.wiki_dir).contains("DROPPED-SECRET"), "wiki file");
    assert!(!bytes(&f.hub_dir).contains("DROPPED-SECRET"), "hub folder");
}
