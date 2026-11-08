//! What a review found: history beside a writer, a lost record of a state, a purge that stops half
//! way, long words in snippets, decomposed letters, a backend that cannot purge.
//!
//! Pattern: Example-based integration tests over a real in-memory store with failure and delay injection.

mod common;

use std::sync::atomic::Ordering;

use common::revfail::make;
use common::{doc_with, say, wiki};
use fs_objects::{ActivityFilter, Store};
use fs_wiki_core::{Error, Expect, SearchQuery, Texts};

fn has(doc: &fs_text::Document, needle: &str) -> bool {
    serde_json::to_string(doc).unwrap().contains(needle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_beside_a_writer_never_invents_a_revision() {
    let (rf, w, _) = make().await;
    let t = w.create("a", "zero\n").await.unwrap();
    rf.delay_ms.store(15, Ordering::SeqCst);
    let (w2, id) = (w.clone(), t.id);
    let writer = tokio::spawn(async move {
        for i in 0..15 {
            let cur = w2.get(id).await.unwrap().unwrap();
            w2.edit(id, doc_with(&cur, &format!("n{i}\n")))
                .await
                .unwrap();
        }
    });
    let w3 = w.clone();
    let reader = tokio::spawn(async move {
        for _ in 0..150 {
            w3.history(id, None, 5).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(3)).await;
        }
    });
    writer.await.unwrap();
    reader.await.unwrap();
    rf.delay_ms.store(0, Ordering::SeqCst);
    let all = w.history(id, None, 100).await.unwrap();
    assert_eq!(
        all.items.len(),
        16,
        "one revision per write: {:?}",
        all.items.iter().map(|i| &i.summary).collect::<Vec<_>>()
    );
    assert!(all.items.iter().all(|i| i.author == common::ME));
}

#[tokio::test]
async fn a_state_whose_record_failed_is_kept_by_the_next_write() {
    let (rf, w, _) = make().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    rf.revisions_left.store(0, Ordering::SeqCst);
    assert!(w.edit(t.id, doc_with(&t, "lost-state\n")).await.is_err());
    rf.revisions_left.store(usize::MAX, Ordering::SeqCst);
    let now = w.get(t.id).await.unwrap().unwrap();
    assert!(has(&now.document, "lost-state"), "the text was written");
    let t = say(&w, &now, "four\n").await;
    let page = w.history(t.id, None, 20).await.unwrap();
    let numbers: Vec<u64> = page.items.iter().map(|i| i.number).collect();
    assert_eq!(numbers, [4, 3, 2, 1]);
    let kept = w.get_revision(t.id, 3).await.unwrap().unwrap();
    assert!(has(&kept.document, "lost-state"), "no state is lost");
    assert!(page.items[1].summary.starts_with("state recorded late"));
}

#[tokio::test]
async fn a_text_without_any_record_gets_its_state_as_revision_one() {
    let (rf, w, _) = make().await;
    rf.revisions_left.store(0, Ordering::SeqCst);
    assert!(
        w.create("old", "before history\n").await.is_err(),
        "its record failed"
    );
    rf.revisions_left.store(usize::MAX, Ordering::SeqCst);
    let t = w
        .get_by_path("old")
        .await
        .unwrap()
        .expect("the text exists");
    let t = say(&w, &t, "after\n").await;
    let page = w.history(t.id, None, 10).await.unwrap();
    assert_eq!(
        page.items.iter().map(|i| i.number).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(page.items[1].summary, "state before history began");
    let first = w.get_revision(t.id, 1).await.unwrap().unwrap();
    assert!(has(&first.document, "before history"));
    // The current state is never removed, whatever the numbers say.
    let err = w.purge_revision(t.id, 2).await.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Invalid {
                issue: "current-revision",
                ..
            }
        ),
        "{err:?}"
    );
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
    // The record of the removal is written twice; the second write fails.
    let left = rf.revisions_left.load(Ordering::SeqCst);
    assert_eq!(left, usize::MAX);
    rf.revisions_left.store(1, Ordering::SeqCst);
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

#[tokio::test]
async fn a_backend_that_cannot_purge_does_not_say_it_can() {
    let store = std::sync::Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let w = fs_wiki_core::WikiRepo::with_backend(store.clone(), store, common::ME);
    let caps = w.capabilities();
    assert!(!caps.is_supported("text.trash") && !caps.is_supported("text.history"));
    assert!(caps.is_supported("text.search"));
    let t = w.create("a", "x\n").await.unwrap();
    w.trash(t.id).await.unwrap();
    match w.purge(t.id).await {
        Err(Error::Unsupported(u)) => assert!(u.to_string().contains("text.trash"), "{u}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_word_longer_than_a_snippet_gives_its_beginning_highlighted() {
    let w = wiki().await;
    let long = format!("x{}", "é".repeat(120));
    w.create("b", &format!("{long} tail\n")).await.unwrap();
    let q = SearchQuery {
        term: long,
        ..Default::default()
    };
    let hit = &w.search(&q, None, 10).await.unwrap().items[0];
    let s = hit.snippets.iter().find(|s| s.field == "body").unwrap();
    assert!(s.text.len() <= 240 && !s.text.is_empty());
    assert_eq!(s.highlights.len(), 1);
    assert_eq!(
        (s.highlights[0].start, s.highlights[0].end as usize),
        (0, s.text.len())
    );
    assert!(s.text.is_char_boundary(s.text.len()) && s.cut_after);
}

#[tokio::test]
async fn a_decomposed_letter_finds_a_composed_one() {
    let w = wiki().await;
    w.create("c", "Ma\u{0308}rchen\n").await.unwrap();
    w.create("d", "M\u{00e4}rchen too\n").await.unwrap();
    let q = SearchQuery {
        term: "M\u{00e4}rchen".into(),
        ..Default::default()
    };
    assert_eq!(w.search(&q, None, 10).await.unwrap().items.len(), 2);
    let q = SearchQuery {
        term: "Ma\u{0308}rchen".into(),
        ..Default::default()
    };
    assert_eq!(w.search(&q, None, 10).await.unwrap().items.len(), 2);
    let _ = Expect::Any;
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
