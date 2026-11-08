//! Spaces and folders: what they own, the rules for names and paths, rename and delete.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::wiki;
use fs_objects::ObjectId;
use fs_wiki_core::{Error, NodeKind, Reason, Texts, WikiRepo};

async fn names(w: &WikiRepo, parent: Option<ObjectId>) -> Vec<String> {
    let page = w.children(parent, 0, 100).await.unwrap();
    page.items.iter().map(|n| n.name.clone()).collect()
}

#[tokio::test]
async fn a_space_and_a_folder_own_their_prefix_and_pages_find_their_parent() {
    let w = wiki().await;
    let space = w.create_folder(None, "Manual", "").await.unwrap();
    assert_eq!(
        (space.kind, space.path.as_str(), space.parent),
        (NodeKind::Space, "manual", None)
    );
    let guide = w
        .create_folder(Some(space.id), "User guide", "")
        .await
        .unwrap();
    assert_eq!(
        (guide.kind, guide.path.as_str()),
        (NodeKind::Folder, "manual/user-guide")
    );
    assert_eq!(guide.parent, Some(space.id));
    let page = w
        .create("manual/user-guide/start", "# Start\n")
        .await
        .unwrap();
    w.create("manual/intro", "# Intro\n").await.unwrap();
    w.create("outside", "# Out\n").await.unwrap();
    assert_eq!(names(&w, Some(guide.id)).await, ["Start"]);
    assert_eq!(
        names(&w, Some(space.id)).await,
        ["User guide", "Intro"].map(String::from),
        "folders and pages, never placed: creation order"
    );
    assert_eq!(names(&w, None).await, ["Manual", "Out"].map(String::from));
    let got = w.get_folder(guide.id).await.unwrap().unwrap();
    assert_eq!((got.id, got.name.as_str()), (guide.id, "User guide"));
    assert!(
        w.get_folder(page.id).await.unwrap().is_none(),
        "a page is not a folder"
    );
    let kids = w.children(Some(guide.id), 0, 10).await.unwrap().items;
    assert_eq!(
        (kids[0].id, kids[0].kind, kids[0].parent),
        (page.id, NodeKind::Page, Some(guide.id))
    );
}

#[tokio::test]
async fn folder_names_and_paths_follow_the_rules() {
    let w = wiki().await;
    let space = w.create_folder(None, "Docs", "docs").await.unwrap();
    let bad = |e: Error| match e {
        Error::InvalidPath { reason, .. } => reason,
        other => panic!("not a path error: {other:?}"),
    };
    let r = w
        .create_folder(Some(space.id), "Привет", "")
        .await
        .unwrap_err();
    assert_eq!(bad(r), Reason::EmptySlug);
    let ok = w
        .create_folder(Some(space.id), "Привет", "docs/privet")
        .await
        .unwrap();
    assert_eq!(ok.name, "Привет");
    for (path, reason) in [
        ("other/x", Reason::OutsideParent),
        ("docs", Reason::OutsideParent),
        ("docs/../x", Reason::DotSegment),
        ("docs//x", Reason::EmptySegment),
        ("/docs/x", Reason::EmptySegment),
        ("docs/x/", Reason::EmptySegment),
    ] {
        let e = w
            .create_folder(Some(space.id), "N", path)
            .await
            .unwrap_err();
        assert_eq!(bad(e), reason, "{path}");
    }
    let e = w.create_folder(None, "A", "a/../b").await.unwrap_err();
    assert_eq!(bad(e), Reason::DotSegment);
    let long = "n".repeat(256);
    assert!(matches!(
        w.create_folder(None, &long, "long").await,
        Err(Error::Invalid {
            issue: "name-too-long",
            ..
        })
    ));
    assert!(matches!(
        w.create_folder(None, "a/b", "ab").await,
        Err(Error::Invalid {
            issue: "name-has-slash",
            ..
        })
    ));
    let taken = w
        .create_folder(None, "Docs again", "docs")
        .await
        .unwrap_err();
    assert!(
        matches!(taken, Error::PathTaken(ref p) if p == "docs"),
        "{taken:?}"
    );
    let nothing = w
        .create_folder(Some(ObjectId::new()), "N", "")
        .await
        .unwrap_err();
    assert!(matches!(nothing, Error::NoSuchNode(_)));
}

#[tokio::test]
async fn a_folder_over_existing_texts_adopts_them() {
    let w = wiki().await;
    let a = w.create("wiki/a", "a\n").await.unwrap();
    let b = w.create("wiki/sub/b", "b\n").await.unwrap();
    let space = w.create_folder(None, "Wiki", "wiki").await.unwrap();
    assert_eq!(
        w.get(a.id).await.unwrap().unwrap().path,
        "wiki/a",
        "the texts stay"
    );
    assert_eq!(w.get(b.id).await.unwrap().unwrap().path, "wiki/sub/b");
    let kids: Vec<_> = w
        .children(Some(space.id), 0, 10)
        .await
        .unwrap()
        .items
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(
        kids,
        [a.id, b.id],
        "b has no folder of its own: its parent is the space"
    );
}

#[tokio::test]
async fn rename_changes_only_the_name_and_delete_needs_an_empty_folder() {
    let w = wiki().await;
    let space = w.create_folder(None, "Docs", "docs").await.unwrap();
    let renamed = w.rename_folder(space.id, "Documents").await.unwrap();
    assert_eq!(
        (renamed.name.as_str(), renamed.path.as_str()),
        ("Documents", "docs")
    );
    let t = w.create("docs/a", "a\n").await.unwrap();
    assert!(matches!(w.delete_folder(space.id).await, Err(Error::NotEmpty(id)) if id == space.id));
    w.trash(t.id).await.unwrap();
    assert!(
        matches!(w.delete_folder(space.id).await, Err(Error::NotEmpty(_))),
        "a trashed text still blocks"
    );
    w.purge(t.id).await.unwrap();
    let inner = w.create_folder(Some(space.id), "In", "").await.unwrap();
    assert!(
        matches!(w.delete_folder(space.id).await, Err(Error::NotEmpty(_))),
        "a folder inside blocks"
    );
    w.delete_folder(inner.id).await.unwrap();
    w.delete_folder(space.id).await.unwrap();
    assert!(w.get_folder(space.id).await.unwrap().is_none());
    assert!(matches!(
        w.delete_folder(space.id).await,
        Err(Error::NoSuchNode(_))
    ));
    assert!(matches!(
        w.rename_folder(space.id, "x").await,
        Err(Error::NoSuchNode(_))
    ));
}
