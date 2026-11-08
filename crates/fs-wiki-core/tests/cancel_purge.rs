//! A purge that is dropped half way (a timeout, a client that goes away) leaves a store that reads right,
//! the clean-up reminder set, and a retry that completes with the secret gone from the file bytes.
//!
//! Pattern: Example-based integration tests with cancel injection — the purge future is polled a fixed
//! number of times and then dropped, for every count up to completion, over a store in a file.

mod common;

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use async_trait::async_trait;
use common::ports::DirectPort;
use common::{say, ME};
use fs_objects::{ObjectId, Query, Store, TursoStore};
use fs_wiki_core::{Error, LogPurge, PurgeOutcome, Refusal, Texts, WikiRepo};

const SECRET: &str = "CANCEL-SECRET";

/// A port that gives control back before and after each call, so the wiki's own steps are cancel points.
struct Yielding(DirectPort);

#[async_trait]
impl LogPurge for Yielding {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        tokio::task::yield_now().await;
        let r = self.0.purge(id).await;
        tokio::task::yield_now().await;
        r
    }
    async fn clean_files(&self) -> Result<bool, Refusal> {
        tokio::task::yield_now().await;
        self.0.clean_files().await
    }
}

/// Poll `fut` at most `polls` times (yielding to the runtime between), then drop it. Some = it finished.
async fn drop_after<F: Future>(fut: F, polls: usize) -> Option<F::Output> {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..polls {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return Some(v);
        }
        tokio::task::yield_now().await;
    }
    None
}

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

async fn setup(dir: &std::path::Path, yielding: bool) -> (WikiRepo, Arc<dyn Store>, ObjectId) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("objects.db")).await.unwrap());
    let port: Arc<dyn LogPurge> = if yielding {
        Arc::new(Yielding(DirectPort(store.clone())))
    } else {
        Arc::new(DirectPort(store.clone()))
    };
    let w = WikiRepo::new(store.clone(), ME).with_log_purge(port);
    let t = w.create("a", &format!("{SECRET} one\n")).await.unwrap();
    let t = say(&w, &t, &format!("{SECRET} two\n")).await;
    let t = say(&w, &t, "three\n").await;
    w.trash(t.id).await.unwrap();
    (w, store, t.id)
}

/// Everything the wiki keeps of texts: the text and its revision records (not the clean-up reminder).
async fn records(store: &Arc<dyn Store>) -> usize {
    let all = store.query(&Query::new()).await.unwrap();
    all.iter()
        .filter(|o| o.envelope.object_type != "CleanPending")
        .count()
}

/// Whether the clean-up reminder is set.
async fn reminder(store: &Arc<dyn Store>) -> bool {
    let q = Query::new().of_type("CleanPending");
    let all = store.query(&q).await.unwrap();
    all.iter().any(|o| o.content.to_string().contains("true"))
}

async fn check(
    w: &WikiRepo,
    store: &Arc<dyn Store>,
    id: ObjectId,
    dir: &std::path::Path,
    k: usize,
) {
    // The store reads right: in the trash with its whole history, or gone with all records.
    let in_trash = w.list_trash(None, 10).await.unwrap().items;
    match in_trash.as_slice() {
        [item] => {
            assert_eq!(item.id, id, "k={k}");
            let h = w.history(id, None, 10).await.unwrap();
            assert!(!h.items.is_empty(), "k={k}: history of a trashed text");
        }
        [] => {
            assert_eq!(
                records(store).await,
                0,
                "k={k}: records left after the text is gone"
            );
            if bytes(dir).contains(SECRET) {
                assert!(reminder(store).await, "k={k}: secret on disk, no reminder");
            }
        }
        _ => panic!("k={k}"),
    }
    // The write lock is free and writes work.
    w.create("later", "x\n")
        .await
        .unwrap_or_else(|e| panic!("k={k}: {e:?}"));
    // A retry completes (or says the text is gone) and the secret leaves the file.
    match w.purge(id).await {
        Ok(()) | Err(Error::NotFound(_)) => {}
        Err(e) => panic!("k={k}: retry {e:?}"),
    }
    assert!(w.retry_file_cleanup().await.unwrap(), "k={k}");
    assert!(!bytes(dir).contains(SECRET), "k={k}: secret in the file");
    assert!(
        w.list_trash(None, 10).await.unwrap().items.is_empty(),
        "k={k}"
    );
}

async fn sweep(yielding: bool) {
    let mut finished_at = None;
    for k in 0..400 {
        let dir = tempfile::tempdir().unwrap();
        let (w, store, id) = setup(dir.path(), yielding).await;
        let done = drop_after(w.purge(id), k).await;
        check(&w, &store, id, dir.path(), k).await;
        if done.is_some() {
            finished_at = Some(k);
            break;
        }
    }
    let k = finished_at.expect("the purge never finished within 400 polls");
    println!("yielding={yielding}: purge finishes after {k} polls");
}

#[tokio::test]
async fn a_purge_dropped_at_any_poll_leaves_a_retryable_store() {
    sweep(false).await;
}

#[tokio::test]
async fn a_purge_dropped_between_the_wikis_own_steps_leaves_a_retryable_store() {
    sweep(true).await;
}

#[tokio::test]
async fn a_purged_revision_dropped_at_any_poll_is_repeatable() {
    for k in 0..400 {
        let dir = tempfile::tempdir().unwrap();
        let (w, store, id) = setup(dir.path(), true).await;
        w.restore_from_trash(id, "").await.unwrap();
        let done = drop_after(w.purge_revision(id, 1), k).await;
        w.get(id).await.unwrap().expect("text still reads");
        let left = bytes(dir.path()).contains("CANCEL-SECRET one");
        match w.purge_revision(id, 1).await {
            Ok(()) => {}
            // Already completed: the tombstone answers "gone".
            Err(Error::NoSuchRevision { .. }) => {
                assert!(
                    !left || reminder(&store).await,
                    "k={k}: secret on disk, no reminder"
                );
            }
            Err(e) => panic!("k={k}: retry {e:?}"),
        }
        assert!(w.retry_file_cleanup().await.unwrap());
        assert!(!bytes(dir.path()).contains("CANCEL-SECRET one"), "k={k}");
        if done.is_some() {
            return;
        }
    }
    panic!("never finished");
}
