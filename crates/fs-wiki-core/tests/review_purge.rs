//! Purges that stop half way or are refused: the logs go first, a retry completes, a refusing port
//! changes nothing, no record above a gap is missed.
//!
//! Pattern: Example-based integration tests over a real in-memory store with failure injection and a
//! stand-in for the purge port.

mod common;

use std::sync::atomic::Ordering;

use common::revfail::make;
use common::{doc_with, say};
use fs_objects::{ActivityFilter, Store};
use fs_wiki_core::{Error, Texts};

fn has(doc: &fs_text::Document, needle: &str) -> bool {
    serde_json::to_string(doc).unwrap().contains(needle)
}

async fn log_text(store: &fs_objects::TursoStore) -> String {
    let all = store.activities(&ActivityFilter::default()).await.unwrap();
    serde_json::to_string(&all.iter().map(|a| &a.payload).collect::<Vec<_>>()).unwrap()
}

#[tokio::test]
async fn a_purge_of_a_revision_that_stopped_half_way_is_finished_by_a_retry() {
    let (rf, w, store) = make().await;
    let t = w.create("a", "half-needle\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    let t = say(&w, &t, "three\n").await;
    // The logs are purged first, then the record is written; that write fails.
    rf.revisions_left.store(0, Ordering::SeqCst);
    assert!(w.purge_revision(t.id, 1).await.is_err());
    rf.revisions_left.store(usize::MAX, Ordering::SeqCst);
    w.purge_revision(t.id, 1).await.unwrap();
    assert!(!log_text(&store).await.contains("half-needle"));
    assert!(w.get_revision(t.id, 1).await.unwrap().is_none());
    assert!(matches!(
        w.purge_revision(t.id, 1).await,
        Err(Error::NoSuchRevision { .. })
    ));
    assert_eq!(w.history(t.id, None, 10).await.unwrap().items.len(), 2);
}

#[tokio::test]
async fn a_purge_of_a_text_that_stopped_half_way_is_finished_by_a_retry() {
    let (rf, w, store) = make().await;
    let t = w.create("a", "trash-needle\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    w.trash(t.id).await.unwrap();
    rf.fail_delete.store(true, Ordering::SeqCst);
    assert!(w.purge(t.id).await.is_err());
    w.purge(t.id).await.unwrap();
    assert!(!log_text(&store).await.contains("trash-needle"));
    assert!(w.list_trash(None, 10).await.unwrap().items.is_empty());
    assert!(store.get(t.id).await.unwrap().is_none());
}

/// Every document and every log entry of the store, as one text.
async fn everything(store: &fs_objects::TursoStore) -> String {
    let mut all = log_text(store).await;
    let objects = fs_objects::traits::ObjectQuery::query(store, &fs_objects::Query::new())
        .await
        .unwrap();
    for o in objects {
        all.push_str(&o.content.to_string());
    }
    all
}

/// Five writes, a store undo of three (the hint goes back, the newer records stay), then the trash.
async fn undone_and_trashed() -> (
    std::sync::Arc<common::revfail::RevFail>,
    std::sync::Arc<fs_wiki_core::WikiRepo>,
    std::sync::Arc<fs_objects::TursoStore>,
    fs_objects::ObjectId,
) {
    let (rf, w, store) = make().await;
    let mut t = w.create("a", "SECRET0\n").await.unwrap();
    for i in 1..5 {
        t = say(&w, &t, &format!("SECRET{i}\n")).await;
    }
    store.undo(common::ME, 3).await.unwrap();
    w.trash(t.id).await.unwrap();
    (rf, w, store, t.id)
}

fn left(all: &str) -> Vec<usize> {
    (0..5)
        .filter(|i| all.contains(&format!("SECRET{i}")))
        .collect()
}

#[tokio::test]
async fn purge_after_a_store_undo_leaves_no_record_above_the_hint() {
    let (_, w, store, id) = undone_and_trashed().await;
    w.purge(id).await.unwrap();
    assert_eq!(left(&everything(&store).await), Vec::<usize>::new());
}

#[tokio::test]
async fn a_retried_purge_after_a_store_undo_reaches_the_records_above_a_gap() {
    let (rf, w, store, id) = undone_and_trashed().await;
    rf.deletes_left.store(2, Ordering::SeqCst);
    assert!(w.purge(id).await.is_err(), "the third delete fails");
    rf.deletes_left.store(usize::MAX, Ordering::SeqCst);
    w.purge(id).await.unwrap();
    assert_eq!(left(&everything(&store).await), Vec::<usize>::new());
}

#[tokio::test]
async fn a_request_that_is_refused_writes_nothing() {
    let (rf, w, _) = make().await;
    let t = w.create("a", "x\n").await.unwrap();
    let t = say(&w, &t, "y\n").await;
    rf.revisions_left.store(0, Ordering::SeqCst);
    assert!(
        w.edit(t.id, doc_with(&t, "z\n")).await.is_err(),
        "its record fails: the state is unrecorded"
    );
    rf.revisions_left.store(usize::MAX, Ordering::SeqCst);
    assert!(matches!(
        w.purge_revision(t.id, 9).await,
        Err(Error::NoSuchRevision { .. })
    ));
    assert!(
        matches!(w.purge_revision(t.id, 2).await, Ok(())),
        "an old one can go"
    );
    let numbers: Vec<u64> = w
        .history(t.id, None, 10)
        .await
        .unwrap()
        .items
        .iter()
        .map(|i| i.number)
        .collect();
    assert_eq!(
        numbers,
        [3, 1],
        "the late record came with the valid request only"
    );
}

fn hubless(
    w: &fs_wiki_core::WikiRepo,
    id: fs_objects::ObjectId,
) -> impl std::future::Future<Output = Result<(), Error>> + '_ {
    w.purge(id)
}

#[tokio::test]
async fn a_port_that_refuses_purges_nothing_and_a_retry_completes() {
    let store = std::sync::Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let as_store: std::sync::Arc<dyn Store> = store.clone();
    let gate = common::ports::Gate::over(as_store.clone());
    let w = fs_wiki_core::WikiRepo::new(as_store, common::ME).with_log_purge(gate.clone());
    let t = w.create("a", "SECRET0\n").await.unwrap();
    let t = say(&w, &t, "SECRET1\n").await;
    let t = say(&w, &t, "SECRET2\n").await;
    // purgeRevision: refused, then nothing changed and the record still reads
    for refusal in [
        fs_wiki_core::Refusal::NoHub,
        fs_wiki_core::Refusal::NotSynced(t.id),
    ] {
        gate.set(Some(refusal.clone()));
        let err = w.purge_revision(t.id, 1).await.unwrap_err();
        assert!(
            matches!(&err, Error::PurgeRefused(r) if *r == refusal),
            "{err:?}"
        );
        let kept = w
            .get_revision(t.id, 1)
            .await
            .unwrap()
            .expect("the record is untouched");
        assert!(has(&kept.document, "SECRET0"));
    }
    gate.set(None);
    w.purge_revision(t.id, 1).await.unwrap();
    assert!(!log_text(&store).await.contains("SECRET0"));
    assert!(w.get_revision(t.id, 1).await.unwrap().is_none());
    // purge of a trashed text: refused before anything is deleted, then completed
    w.trash(t.id).await.unwrap();
    gate.set(Some(fs_wiki_core::Refusal::NoHub));
    let err = hubless(&w, t.id).await.unwrap_err();
    assert!(matches!(
        err,
        Error::PurgeRefused(fs_wiki_core::Refusal::NoHub)
    ));
    assert_eq!(
        w.list_trash(None, 10).await.unwrap().items.len(),
        1,
        "still in the trash"
    );
    assert!(
        w.history(t.id, None, 10).await.is_ok(),
        "its history is still there"
    );
    gate.set(None);
    w.purge(t.id).await.unwrap();
    assert!(!everything(&store).await.contains("SECRET"));
    assert!(w.list_trash(None, 10).await.unwrap().items.is_empty());
}
