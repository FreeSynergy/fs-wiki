//! What a review found: history beside a writer, a lost record of a state, a purge that stops half
//! way, long words in snippets, decomposed letters, a backend that cannot purge.
//!
//! Pattern: Example-based integration tests over a real in-memory store with failure and delay injection.

mod common;

use std::sync::atomic::Ordering;

use common::revfail::make;
use common::{doc_with, say, wiki};
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
