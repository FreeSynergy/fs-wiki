//! A folder move that is repeated, aimed at an occupied folder, or retried with another target.
//!
//! Pattern: Example-based integration tests over a real in-memory store, with failures injected.

mod common;

use std::sync::Arc;

use common::{everything, failing, fill, paths, setup, wiki, ME};
use fs_objects::TursoStore;
use fs_wiki_core::{Error, Texts, WikiRepo};

#[tokio::test]
async fn repeating_a_completed_move_reports_it_and_writes_nothing() {
    let (w, store) = setup().await;
    fill(&w).await;
    let first = w.move_folder("manual", "guide").await.unwrap();
    let state = store.undo_state(ME).await.unwrap();
    let again = w.move_folder("manual", "guide").await.unwrap();
    assert_eq!(paths(&again), paths(&first));
    assert_eq!(store.undo_state(ME).await.unwrap(), state);
    // Another move from the now empty folder is still an error.
    assert!(matches!(
        w.move_folder("manual", "elsewhere").await,
        Err(Error::NoSuchFolder(_))
    ));
}

#[tokio::test]
async fn a_move_onto_a_folder_with_texts_is_refused_and_writes_nothing() {
    let (w, store) = setup().await;
    fill(&w).await;
    w.create("target/other", "t").await.unwrap();
    let before = everything(&w).await;
    let state = store.undo_state(ME).await.unwrap();
    let err = w.move_folder("manual", "target").await.unwrap_err();
    assert!(matches!(err, Error::TargetNotEmpty { .. }), "{err:?}");
    assert_eq!(everything(&w).await, before);
    assert_eq!(store.undo_state(ME).await.unwrap(), state);
}

#[tokio::test]
async fn an_unfinished_move_is_not_split_by_a_retry_with_another_target() {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let healthy = WikiRepo::new(store.clone(), ME);
    fill(&healthy).await;
    // Stop after the record, the two outside texts and the first moved text.
    let broken = failing(&store, 4);
    assert!(broken.move_folder("manual", "guide").await.is_err());
    let before = everything(&healthy).await;

    let err = healthy
        .move_folder("manual", "other-place")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::MoveUnfinished { .. }), "{err:?}");
    assert_eq!(everything(&healthy).await, before, "nothing was written");

    // The same target finishes it.
    healthy.move_folder("manual", "guide").await.unwrap();
    let all = everything(&healthy).await;
    assert!(paths(&all).iter().all(|p| !p.starts_with("manual/")));
    // And now another move is fine again.
    healthy.move_folder("guide", "other-place").await.unwrap();
}

#[tokio::test]
async fn a_path_with_the_last_code_point_is_refused_so_no_text_stays_behind() {
    let w = wiki().await;
    let err = w.create("a/\u{10FFFF}", "x").await.unwrap_err();
    assert!(matches!(err, Error::InvalidPath { .. }), "{err:?}");
    w.create("a/b", "x").await.unwrap();
    w.create("a/\u{10FFFE}z", "x").await.unwrap();
    let moved = w.move_folder("a", "c").await.unwrap();
    assert_eq!(moved.len(), 2);
    assert_eq!(
        w.list("a/", fs_wiki_core::Paging::default()).await.unwrap(),
        [] as [fs_wiki_core::TextEntry; 0]
    );
}

#[tokio::test]
async fn a_move_interrupted_at_any_write_ends_as_the_undisturbed_one_with_candidate_links() {
    async fn fill_with_links(w: &WikiRepo) {
        for (path, md) in [
            ("t/a.md", "[b](b) [out](../s/p) [self](./a)\n"),
            ("t/b.md", "[a](a) [idx](sub/) [up](../s/p.md)\n"),
            ("t/sub/index.md", "[a](../a)\n"),
            ("s/p.md", "[a](../t/a) [b](../t/b.md) [s](../t/sub/)\n"),
            ("s/other.md", "[x](../t/a.md#h)\n"),
        ] {
            w.create(path, md).await.unwrap();
        }
    }
    let reference = wiki().await;
    fill_with_links(&reference).await;
    reference.move_folder("t", "deep/u").await.unwrap();
    let mut expected = vec![];
    for t in everything(&reference).await {
        expected.push((t.path.clone(), common::md(&reference, t.id).await));
    }
    for fail_at in 0..7 {
        let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
        let healthy = WikiRepo::new(store.clone(), ME);
        fill_with_links(&healthy).await;
        let broken = failing(&store, fail_at);
        assert!(
            broken.move_folder("t", "deep/u").await.is_err(),
            "at {fail_at}"
        );
        healthy.move_folder("t", "deep/u").await.unwrap();
        let mut got = vec![];
        for t in everything(&healthy).await {
            got.push((t.path.clone(), common::md(&healthy, t.id).await));
        }
        assert_eq!(got, expected, "interrupted at write {fail_at}");
    }
}

#[tokio::test]
async fn a_page_move_interrupted_after_the_links_changed_is_finished_by_repeating_it() {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let healthy = WikiRepo::new(store.clone(), ME);
    let page = healthy.create("page.md", "p").await.unwrap();
    let s = healthy
        .create("s/src.md", "[p](../page) [q](../page.md#h)\n")
        .await
        .unwrap();
    // The link in `s/src.md` is written first (write 0), the page itself last (write 1).
    let broken = failing(&store, 1);
    assert!(broken.move_to(page.id, "other/page.md").await.is_err());
    healthy.move_to(page.id, "other/page.md").await.unwrap();
    assert_eq!(
        common::md(&healthy, s.id).await,
        "[p](../other/page) [q](../other/page.md#h)\n"
    );
}
