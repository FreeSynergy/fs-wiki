//! The trash: out of sight but kept with its history, restored, or removed for good.
//!
//! Pattern: Example-based integration tests over a real in-memory store; the purge is checked in the
//! activity log of the store itself, where the content of an object is kept as before/after images.

mod common;

use common::{paths, say, setup, wiki};
use fs_objects::ActivityFilter;
use fs_wiki_core::{Error, Paging, SearchQuery, Texts};

#[tokio::test]
async fn a_trashed_text_is_out_of_sight_and_its_path_is_free() {
    let w = wiki().await;
    let a = w.create("a", "see [[b]] needle\n").await.unwrap();
    w.create("b", "x\n").await.unwrap();
    let item = w.trash(a.id).await.unwrap();
    assert_eq!((item.id, item.path.as_str()), (a.id, "a"));
    assert!(!item.name.is_empty() && item.size > 0);
    assert!(w.get(a.id).await.unwrap().is_none());
    assert!(w.get_by_path("a").await.unwrap().is_none());
    assert_eq!(paths(&w.list("", Paging::default()).await.unwrap()), ["b"]);
    assert_eq!(
        w.find("needle", Paging::default()).await.unwrap(),
        [] as [fs_wiki_core::TextEntry; 0]
    );
    assert!(
        w.backlinks("b").await.unwrap().is_empty(),
        "its links are out of the index"
    );
    assert!(w.links(a.id).await.is_err());
    let hits = w
        .search(
            &SearchQuery {
                term: "needle".into(),
                ..Default::default()
            },
            None,
            10,
        )
        .await
        .unwrap();
    assert_eq!(hits.items, [] as [fs_wiki_core::SearchHit; 0]);
    w.create("a", "a new text at the freed path\n")
        .await
        .unwrap();
    assert!(
        matches!(w.trash(a.id).await, Err(Error::NotFound(_))),
        "already trashed"
    );
    assert!(matches!(
        w.trash(fs_objects::ObjectId::new()).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn the_trash_lists_most_recent_first_in_pages_and_keeps_history() {
    let w = wiki().await;
    let mut ids = vec![];
    for p in ["a", "b", "c"] {
        let t = w.create(p, "one\n").await.unwrap();
        let t = say(&w, &t, "two\n").await;
        ids.push(t.id);
    }
    for id in &ids {
        w.trash(*id).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    }
    let first = w.list_trash(None, 2).await.unwrap();
    assert_eq!(
        first.items.iter().map(|i| i.id).collect::<Vec<_>>(),
        [ids[2], ids[1]]
    );
    assert!(first.more);
    let rest = w
        .list_trash(Some(&first.items[1].place()), 2)
        .await
        .unwrap();
    assert_eq!(
        rest.items.iter().map(|i| i.id).collect::<Vec<_>>(),
        [ids[0]]
    );
    assert!(!rest.more);
    let history = w.history(ids[0], None, 10).await.unwrap();
    assert_eq!(
        history.items.len(),
        2,
        "history is readable for a trashed text"
    );
    assert!(w.get_revision(ids[0], 1).await.unwrap().is_some());
}

#[tokio::test]
async fn restoring_brings_the_text_back_at_its_path_or_another() {
    let w = wiki().await;
    let a = w.create("a", "one\n").await.unwrap();
    w.trash(a.id).await.unwrap();
    let blocker = w.create("a", "taken\n").await.unwrap();
    let err = w.restore_from_trash(a.id, "").await.unwrap_err();
    assert!(
        matches!(err, Error::PathTaken(ref p) if p == "a"),
        "{err:?}"
    );
    assert_eq!(
        w.list_trash(None, 10).await.unwrap().items.len(),
        1,
        "nothing changed"
    );
    let back = w.restore_from_trash(a.id, "elsewhere/a").await.unwrap();
    assert_eq!((back.id, back.path.as_str()), (a.id, "elsewhere/a"));
    assert_eq!(back.document, a.document);
    assert_eq!(
        w.list_trash(None, 10).await.unwrap().items,
        [] as [fs_wiki_core::TrashItem; 0]
    );
    assert!(matches!(
        w.restore_from_trash(a.id, "").await,
        Err(Error::NotInTrash(_))
    ));
    assert!(matches!(
        w.restore_from_trash(blocker.id, "").await,
        Err(Error::NotInTrash(_))
    ));
    let bad = w
        .trash(blocker.id)
        .await
        .and(w.restore_from_trash(blocker.id, "x//y").await);
    assert!(matches!(bad, Err(Error::InvalidPath { .. })));
}

#[tokio::test]
async fn purge_takes_the_text_out_of_the_store_and_its_log() {
    let (w, store) = setup().await;
    let a = w.create("a", "plain-needle-one see [[b]]\n").await.unwrap();
    let a = say(&w, &a, "plain-needle-two\n").await;
    let keep = w.create("keep", "plain-needle-keep\n").await.unwrap();
    assert!(
        matches!(w.purge(a.id).await, Err(Error::NotInTrash(_))),
        "a live text is not purged"
    );
    w.trash(a.id).await.unwrap();
    w.purge(a.id).await.unwrap();
    assert_eq!(
        w.list_trash(None, 10).await.unwrap().items,
        [] as [fs_wiki_core::TrashItem; 0]
    );
    assert!(matches!(
        w.history(a.id, None, 5).await,
        Err(Error::NotFound(_))
    ));
    assert!(w.get_revision(a.id, 1).await.is_err());
    assert!(
        store.get(a.id).await.unwrap().is_none(),
        "the object is gone"
    );
    let log = serde_json::to_string(
        &store
            .activities(&ActivityFilter::default())
            .await
            .unwrap()
            .iter()
            .map(|x| &x.payload)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(
        !log.contains("plain-needle-one") && !log.contains("plain-needle-two"),
        "not in the log"
    );
    assert!(
        log.contains("plain-needle-keep"),
        "other texts are not touched"
    );
    assert!(w
        .find("plain-needle", Paging::default())
        .await
        .unwrap()
        .iter()
        .all(|t| t.id == keep.id));
    assert!(
        w.backlinks("b").await.unwrap().is_empty(),
        "not in the link index"
    );
    assert!(matches!(
        w.purge(a.id).await,
        Err(Error::NotInTrash(_) | Error::NotFound(_))
    ));
}
