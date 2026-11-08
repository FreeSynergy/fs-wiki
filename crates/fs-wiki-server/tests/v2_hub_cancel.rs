//! A purge that times out or is dropped does not cut the work: against the real hub daemon behind a
//! proxy that delays every chunk, the sync and purge run on in their own task, the wiki's call is
//! answered `Timeout` (or dropped) at a point where the work is certainly not done, and the log of the
//! first revision is gone shortly after, without any retry. A retry then completes, with the secret in
//! neither the wiki's `.db`/`-wal` nor the hub's folder.
//!
//! Pattern: Example-based integration tests with a delaying proxy as the test seam (fails if the
//! purge runs in the caller's future instead of a task of its own).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fs_hub::{Config, Daemon};
use fs_objects::{ActivityFilter, ObjectId, Query, Store, TursoStore};
use fs_wiki_core::{Error, Refusal, Texts, WikiRepo};
use fs_wiki_server::HubPurge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

const ME: &str = "https://example.org/actors/a";
const DELAY: Duration = Duration::from_millis(150);

fn bytes(dir: &Path) -> String {
    let mut out = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            out.push_str(&String::from_utf8_lossy(&std::fs::read(&p).unwrap()));
        }
    }
    out
}

/// Forwards `proxy` to `hub`, holding every chunk back by [`DELAY`]; counts the chunks.
fn delaying_proxy(proxy: &Path, hub: PathBuf, chunks: Arc<AtomicUsize>) {
    std::fs::create_dir_all(proxy.parent().unwrap()).unwrap();
    let listener = UnixListener::bind(proxy).unwrap();
    tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            let Ok(server) = UnixStream::connect(&hub).await else {
                continue;
            };
            let (cr, cw) = client.into_split();
            let (sr, sw) = server.into_split();
            tokio::spawn(pipe(cr, sw, chunks.clone()));
            tokio::spawn(pipe(sr, cw, chunks.clone()));
        }
    });
}

async fn pipe(
    mut from: impl AsyncReadExt + Unpin,
    mut to: impl AsyncWriteExt + Unpin,
    chunks: Arc<AtomicUsize>,
) {
    let mut buf = vec![0u8; 64 * 1024];
    while let Ok(n) = from.read(&mut buf).await {
        if n == 0 {
            break;
        }
        chunks.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(DELAY).await;
        if to.write_all(&buf[..n]).await.is_err() {
            break;
        }
    }
}

struct Fixture {
    w: Arc<WikiRepo>,
    store: Arc<dyn Store>,
    wiki_dir: PathBuf,
    hub_dir: PathBuf,
    hub_socket: PathBuf,
    chunks: Arc<AtomicUsize>,
    _hub: Daemon,
    _tmp: tempfile::TempDir,
}

async fn fixture(purge_limit: Duration) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let wiki_dir = tmp.path().join("wiki");
    let hub_dir = tmp.path().join("hub-data");
    std::fs::create_dir_all(&wiki_dir).unwrap();
    let hub_socket = tmp.path().join("run/hub.sock");
    let hub = Daemon::spawn(Config::new(hub_socket.clone(), hub_dir.clone())).unwrap();
    let chunks = Arc::new(AtomicUsize::new(0));
    let proxy = tmp.path().join("proxy/hub.sock");
    delaying_proxy(&proxy, hub_socket.clone(), chunks.clone());
    let store: Arc<dyn Store> =
        Arc::new(TursoStore::open(wiki_dir.join("objects.db")).await.unwrap());
    let port =
        HubPurge::new(store.clone(), Some(proxy)).with_limits(Duration::from_secs(60), purge_limit);
    let w = Arc::new(WikiRepo::new(store.clone(), ME).with_log_purge(Arc::new(port)));
    Fixture {
        w,
        store,
        wiki_dir,
        hub_dir,
        hub_socket,
        chunks,
        _hub: hub,
        _tmp: tmp,
    }
}

/// A trashed text with a second revision; returns its id and the id of its first revision record
/// (the first log the wiki purges).
async fn trashed(f: &Fixture, secret: &str) -> (ObjectId, ObjectId) {
    let t = f.w.create("a", &format!("{secret}\n")).await.unwrap();
    let mut doc = t.document.clone();
    fs_wiki_core::TextFormat::Markdown.import_external(&mut doc, "second\n");
    f.w.edit(t.id, doc).await.unwrap();
    f.w.trash(t.id).await.unwrap();
    let q = Query::new()
        .of_type("WikiRevision")
        .name_prefix(format!("{}#", t.id));
    let first = f
        .store
        .query(&q)
        .await
        .unwrap()
        .into_iter()
        .find(|o| o.envelope.name.ends_with("#1"))
        .expect("record of revision 1");
    (t.id, first.envelope.id)
}

async fn entries(store: &dyn Store, object: ObjectId) -> usize {
    let filter = ActivityFilter {
        object: Some(object),
        ..Default::default()
    };
    store.activities(&filter).await.unwrap().len()
}

/// Wait (up to 60 s) until the log of `object` is empty.
async fn log_gone(store: &dyn Store, object: ObjectId) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while entries(store, object).await > 0 {
        assert!(Instant::now() < deadline, "the purge did not run on");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// A program over the same store with the normal limit, straight to the hub (no delay).
async fn retry(f: &Fixture, id: ObjectId, secret: &str) {
    let port = HubPurge::new(f.store.clone(), Some(f.hub_socket.clone()));
    let again = WikiRepo::new(f.store.clone(), ME).with_log_purge(Arc::new(port));
    match again.purge(id).await {
        Ok(()) => {}
        Err(e) => panic!("retry: {e:?}"),
    }
    assert_eq!(again.list_trash(None, 10).await.unwrap().items.len(), 0);
    assert!(again.retry_file_cleanup().await.unwrap());
    again.create("after-retry", "x\n").await.unwrap();
    assert!(!bytes(&f.wiki_dir).contains(secret), "wiki file");
    assert!(!bytes(&f.hub_dir).contains(secret), "hub folder");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timeout_stops_the_waiting_not_the_work() {
    let f = fixture(Duration::from_millis(100)).await;
    let (id, rev1) = trashed(&f, "TIMEOUT-SECRET").await;
    assert!(entries(&*f.store, rev1).await > 0);
    // Every chunk takes 150 ms, so the limit of 100 ms is certain to fire first.
    let first = f.w.purge(id).await;
    assert!(
        matches!(first, Err(Error::PurgeRefused(Refusal::Timeout))),
        "{first:?}"
    );
    // No retry: the task goes on by itself and purges the log of the first revision record.
    log_gone(&*f.store, rev1).await;
    // The wiki stopped waiting, so the rest (more logs, the deletes) is for the retry.
    assert_eq!(f.w.list_trash(None, 10).await.unwrap().items.len(), 1);
    f.w.create("later", "x\n").await.unwrap(); // the write lock is free
    retry(&f, id, "TIMEOUT-SECRET").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_caller_that_goes_away_does_not_cut_the_purge() {
    let f = fixture(Duration::from_secs(120)).await;
    // Warm-up: the session is registered, so the next traffic is the purge itself.
    let (warm, _) = trashed(&f, "WARM").await;
    f.w.purge(warm).await.unwrap();
    let (id, rev1) = trashed(&f, "DROPPED-SECRET").await;
    let before = f.chunks.load(Ordering::SeqCst);
    let w = f.w.clone();
    let call = tokio::spawn(async move { w.purge(id).await });
    // Abort only once the sync is on the wire, i.e. after the purge task has started.
    while f.chunks.load(Ordering::SeqCst) == before {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    call.abort();
    let _ = call.await;
    assert!(
        entries(&*f.store, rev1).await > 0,
        "not purged yet at the abort"
    );
    log_gone(&*f.store, rev1).await;
    assert_eq!(f.w.list_trash(None, 10).await.unwrap().items.len(), 1);
    retry(&f, id, "DROPPED-SECRET").await;
}
