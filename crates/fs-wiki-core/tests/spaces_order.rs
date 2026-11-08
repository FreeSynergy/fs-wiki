//! The order of siblings: reorder, paged children, a moved page, a moved folder.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::wiki;
use fs_objects::ObjectId;
use fs_wiki_core::{Error, Placement, Texts, WikiRepo};

async fn names(w: &WikiRepo, parent: Option<ObjectId>) -> Vec<String> {
    let page = w.children(parent, 0, 100).await.unwrap();
    page.items.iter().map(|n| n.name.clone()).collect()
}

async fn joined(w: &WikiRepo, parent: ObjectId) -> String {
    names(w, Some(parent)).await.join("")
}

#[tokio::test]
async fn reorder_places_among_siblings_and_never_touches_the_texts() {
    let w = wiki().await;
    let space = w.create_folder(None, "S", "s").await.unwrap();
    let mut ids = vec![];
    for p in ["s/a", "s/b", "s/c", "s/d"] {
        ids.push(w.create(p, &format!("# {}\n", &p[2..])).await.unwrap().id);
    }
    assert_eq!(joined(&w, space.id).await, "abcd");
    let n = w.reorder(ids[3], Placement::First, None).await.unwrap();
    assert_eq!((n.position, n.id), (0, ids[3]));
    assert_eq!(joined(&w, space.id).await, "dabc");
    w.reorder(ids[3], Placement::Last, None).await.unwrap();
    w.reorder(ids[0], Placement::After, Some(ids[2]))
        .await
        .unwrap();
    assert_eq!(joined(&w, space.id).await, "bcad");
    w.reorder(ids[3], Placement::Before, Some(ids[1]))
        .await
        .unwrap();
    assert_eq!(joined(&w, space.id).await, "dbca");
    let e = w
        .reorder(ids[0], Placement::Before, None)
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        Error::Invalid {
            issue: "anchor",
            ..
        }
    ));
    let e = w
        .reorder(ids[0], Placement::After, Some(space.id))
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            Error::Invalid {
                issue: "anchor",
                ..
            }
        ),
        "not a sibling"
    );
    let e = w
        .reorder(ids[0], Placement::After, Some(ids[0]))
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            Error::Invalid {
                issue: "anchor",
                ..
            }
        ),
        "itself"
    );
    assert!(matches!(
        w.reorder(ObjectId::new(), Placement::First, None).await,
        Err(Error::NotFound(_))
    ));
    let late = w.create("s/e", "# e\n").await.unwrap();
    assert_eq!(
        joined(&w, space.id).await,
        "dbcae",
        "a text never placed comes after the placed ones"
    );
    assert_eq!(w.get(late.id).await.unwrap().unwrap().path, "s/e");
}

#[tokio::test]
async fn children_are_paged_and_a_moved_page_lands_last_in_its_new_folder() {
    let w = wiki().await;
    let s1 = w.create_folder(None, "One", "one").await.unwrap();
    let s2 = w.create_folder(None, "Two", "two").await.unwrap();
    for i in 0..5 {
        w.create(&format!("one/p{i}"), &format!("# P{i}\n"))
            .await
            .unwrap();
    }
    let page = w.children(Some(s1.id), 0, 2).await.unwrap();
    assert!(page.more);
    assert_eq!(
        page.items.iter().map(|n| n.position).collect::<Vec<_>>(),
        [0, 1]
    );
    let page = w.children(Some(s1.id), 4, 2).await.unwrap();
    assert!(!page.more);
    assert_eq!(page.items.len(), 1);
    let a = w.create("two/a", "# A\n").await.unwrap();
    let moved = w.create("one/m", "# M\n").await.unwrap();
    w.reorder(a.id, Placement::First, None).await.unwrap();
    w.move_to(moved.id, "two/m").await.unwrap();
    assert_eq!(names(&w, Some(s2.id)).await.join(""), "AM");
    w.create("two/z", "# Z\n").await.unwrap();
    w.move_to(a.id, "one/a").await.unwrap();
    assert_ne!(names(&w, Some(s1.id)).await, [] as [std::string::String; 0]);
}

#[tokio::test]
async fn moving_a_folder_keeps_its_nodes_and_their_order() {
    let w = wiki().await;
    let space = w.create_folder(None, "Docs", "docs").await.unwrap();
    let inner = w
        .create_folder(Some(space.id), "Inner", "docs/inner")
        .await
        .unwrap();
    let x = w.create("docs/inner/x", "# X\n").await.unwrap();
    let y = w.create("docs/inner/y", "# Y\n").await.unwrap();
    w.reorder(y.id, Placement::First, None).await.unwrap();
    w.move_folder("docs", "manual").await.unwrap();
    let moved = w.get_folder(inner.id).await.unwrap().unwrap();
    assert_eq!(moved.path, "manual/inner");
    assert_eq!(
        w.get_folder(space.id).await.unwrap().unwrap().path,
        "manual"
    );
    assert_eq!(names(&w, Some(inner.id)).await.join(""), "YX");
    assert_eq!(w.get(x.id).await.unwrap().unwrap().path, "manual/inner/x");
    assert_eq!(moved.parent, Some(space.id));
}
