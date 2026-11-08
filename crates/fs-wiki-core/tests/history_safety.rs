//! Revisions that must outlive the store's own history, and the removal of one for good.
//!
//! Pattern: Example-based integration tests over a real in-memory store, plus a count of the rows read
//! for n and 2n texts (a page of history must not cost more for a bigger wiki).

mod common;

use common::counting::Counting;
use common::{say, setup, ME};
use fs_objects::ActivityFilter;
use fs_wiki_core::{Error, Expect, Texts};

#[tokio::test]
async fn numbers_survive_a_store_undo_and_a_compaction() {
    let (w, store) = setup().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    store.undo(ME, 1).await.unwrap();
    let now = w.get(t.id).await.unwrap().unwrap();
    let page = w.history(t.id, None, 10).await.unwrap();
    assert_eq!(
        page.items[0].revision, now.revision,
        "history says what the text is now"
    );
    assert_eq!(page.items[0].number, 3, "a number of its own, none reused");
    let t = say(&w, &now, "three\n").await;
    let numbers =
        |p: fs_wiki_core::RevisionPage| p.items.iter().map(|r| r.number).collect::<Vec<_>>();
    assert_eq!(
        numbers(w.history(t.id, None, 10).await.unwrap()),
        [4, 3, 2, 1],
        "no number was reused"
    );
    store
        .mark_synced(
            store
                .activities(&ActivityFilter::default())
                .await
                .unwrap()
                .last()
                .unwrap()
                .id,
        )
        .await
        .unwrap();
    store.clear_history().await.unwrap();
    let t = say(&w, &t, "four\n").await;
    assert_eq!(
        numbers(w.history(t.id, None, 10).await.unwrap()),
        [5, 4, 3, 2, 1]
    );
    assert_eq!(
        w.get_revision(t.id, 2).await.unwrap().unwrap().info.number,
        2
    );
}

#[tokio::test]
async fn a_revision_is_removed_for_good_but_never_the_current_one() {
    let (w, store) = setup().await;
    let t = w.create("a", "secret-needle one\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    let t = say(&w, &t, "three\n").await;
    let err = w.purge_revision(t.id, 3).await.unwrap_err();
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
    let err = w.purge_revision(t.id, 9).await.unwrap_err();
    assert!(
        matches!(err, Error::NoSuchRevision { number: 9, .. }),
        "{err:?}"
    );
    w.purge_revision(t.id, 1).await.unwrap();
    assert!(w.get_revision(t.id, 1).await.unwrap().is_none());
    let numbers: Vec<u64> = w
        .history(t.id, None, 10)
        .await
        .unwrap()
        .items
        .iter()
        .map(|r| r.number)
        .collect();
    assert_eq!(numbers, [3, 2], "the gap is shown as a gap");
    assert!(matches!(
        w.restore_revision(t.id, 1, Expect::Any).await,
        Err(Error::NoSuchRevision { .. })
    ));
    assert!(matches!(
        w.purge_revision(t.id, 1).await,
        Err(Error::NoSuchRevision { .. })
    ));
    let t = say(&w, &t, "four\n").await;
    assert_eq!(
        w.history(t.id, None, 1).await.unwrap().items[0].number,
        4,
        "numbers go on after the highest"
    );
    let all = store.activities(&ActivityFilter::default()).await.unwrap();
    let log = serde_json::to_string(&all.iter().map(|a| &a.payload).collect::<Vec<_>>()).unwrap();
    assert!(
        !log.contains("secret-needle"),
        "the old text left the log of the store"
    );
}

/// Rows read to show the newest page of history of one text among `n` others.
async fn rows_for(n: usize) -> usize {
    let (counting, repo) = Counting::wiki().await;
    for i in 0..n {
        repo.create(&format!("docs/{i:05}"), "# A page\n\nsome text\n")
            .await
            .unwrap();
    }
    let t = repo.create("mine", "0\n").await.unwrap();
    let t = say(&repo, &t, "1\n").await;
    counting.reset();
    let page = repo.history(t.id, None, 10).await.unwrap();
    assert_eq!(page.items.len(), 2);
    repo.get_revision(t.id, 1).await.unwrap().unwrap();
    counting.rows() + counting.touched()
}

#[tokio::test]
async fn history_costs_the_same_in_a_wiki_twice_as_big() {
    let (small, big) = (rows_for(20).await, rows_for(40).await);
    assert_eq!(
        small, big,
        "a page of history reads what it shows, not the wiki"
    );
}
