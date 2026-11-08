//! Revisions: listing, reading, restoring and writing with a summary.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::{doc_with, md, say, wiki, ME};
use fs_wiki_core::{Error, Expect, Texts, MAX_SUMMARY_BYTES};

#[tokio::test]
async fn every_write_adds_a_numbered_revision_newest_first() {
    let w = wiki().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    let t = w.move_to(t.id, "b").await.unwrap();
    let page = w.history(t.id, None, 10).await.unwrap();
    let numbers: Vec<u64> = page.items.iter().map(|r| r.number).collect();
    assert_eq!(numbers, [3, 2, 1]);
    assert!(!page.more);
    let notes: Vec<&str> = page.items.iter().map(|r| r.summary.as_str()).collect();
    assert_eq!(notes, ["moved", "edited", "created"]);
    assert_eq!(page.items[0].revision, t.revision);
    assert_eq!(page.items[0].path, "b");
    assert_eq!(page.items[2].path, "a");
    assert!(page.items.iter().all(|r| r.author == ME && r.size > 0));
    assert_eq!(page.items[1].restored_from, 0);
}

#[tokio::test]
async fn history_pages_continue_before_a_number() {
    let w = wiki().await;
    let mut t = w.create("a", "0\n").await.unwrap();
    for i in 1..5 {
        t = say(&w, &t, &format!("{i}\n")).await;
    }
    let first = w.history(t.id, None, 2).await.unwrap();
    assert_eq!(
        first.items.iter().map(|r| r.number).collect::<Vec<_>>(),
        [5, 4]
    );
    assert!(first.more);
    let second = w.history(t.id, Some(4), 2).await.unwrap();
    assert_eq!(
        second.items.iter().map(|r| r.number).collect::<Vec<_>>(),
        [3, 2]
    );
    let last = w.history(t.id, Some(2), 2).await.unwrap();
    assert_eq!(last.items.iter().map(|r| r.number).collect::<Vec<_>>(), [1]);
    assert!(!last.more);
}

#[tokio::test]
async fn a_revision_is_read_with_its_document_or_not_found() {
    let w = wiki().await;
    let t = w.create("a", "one\n").await.unwrap();
    say(&w, &t, "two\n").await;
    let old = w.get_revision(t.id, 1).await.unwrap().unwrap();
    assert_eq!(old.info.number, 1);
    assert_eq!(old.document, t.document);
    assert!(w.get_revision(t.id, 9).await.unwrap().is_none());
    assert!(w.get_revision(t.id, 0).await.unwrap().is_none());
    let unknown = fs_objects::ObjectId::new();
    assert!(matches!(
        w.history(unknown, None, 5).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn restoring_writes_the_old_document_as_a_new_revision() {
    let w = wiki().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t2 = say(&w, &t, "two\n").await;
    let moved = w.move_to(t.id, "b").await.unwrap();
    w.set_template(t.id, true, Expect::Any).await.unwrap();
    let back = w.restore_revision(t.id, 1, Expect::Any).await.unwrap();
    assert_eq!(md(&w, t.id).await, "one\n");
    assert_eq!(back.path, "b", "the path stays as it is now");
    assert!(back.template, "flags stay as they are now");
    let page = w.history(t.id, None, 10).await.unwrap();
    assert_eq!(page.items.len(), 5, "nothing was rewritten or cut");
    let newest = &page.items[0];
    assert_eq!((newest.number, newest.restored_from), (5, 1));
    assert_eq!(newest.summary, "restored revision 1");
    assert_ne!(back.revision, t2.revision.max(moved.revision));
    let err = w.restore_revision(t.id, 77, Expect::Any).await.unwrap_err();
    assert!(
        matches!(err, Error::NoSuchRevision { number: 77, .. }),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_stale_expectation_is_a_conflict_that_names_the_current_revision() {
    let w = wiki().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t2 = say(&w, &t, "two\n").await;
    let err = w
        .restore_revision(t.id, 1, Expect::Revision(t.revision))
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Conflict { expected, found, .. } if expected == t.revision && found == t2.revision),
        "{err:?}"
    );
    assert_eq!(md(&w, t.id).await, "two\n", "nothing was written");
    assert_eq!(w.history(t.id, None, 10).await.unwrap().items.len(), 2);
}

#[tokio::test]
async fn edit_with_keeps_the_summary_and_refuses_a_long_one() {
    let w = wiki().await;
    let t = w.create("a", "one\n").await.unwrap();
    let t2 = w
        .edit_with(t.id, Expect::Any, doc_with(&t, "two\n"), "fix typo")
        .await
        .unwrap();
    let t3 = w
        .edit_with(
            t.id,
            Expect::Revision(t2.revision),
            doc_with(&t2, "3\n"),
            "",
        )
        .await
        .unwrap();
    let stale = w
        .edit_with(
            t.id,
            Expect::Revision(t.revision),
            doc_with(&t, "x\n"),
            "late",
        )
        .await;
    assert!(matches!(stale, Err(Error::Conflict { found, .. }) if found == t3.revision));
    let long = "x".repeat(MAX_SUMMARY_BYTES + 1);
    let err = w
        .edit_with(t.id, Expect::Any, doc_with(&t, "y\n"), &long)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::Invalid {
                issue: "summary-too-long",
                ..
            }
        ),
        "{err:?}"
    );
    let page = w.history(t.id, None, 10).await.unwrap();
    assert_eq!(page.items[1].summary, "fix typo");
    assert_eq!(
        page.items[0].summary, "edited",
        "no summary: the provider's own note"
    );
    assert_eq!(page.items.len(), 3);
}
