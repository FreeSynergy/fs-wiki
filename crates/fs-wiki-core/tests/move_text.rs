//! `move`: a new path, and the links in other texts follow (ADR 0042 point 8).

mod common;

use common::{setup, wiki, ME};
use fs_wiki_core::{Error, Texts, MARKDOWN};

async fn md(w: &fs_wiki_core::WikiRepo, id: fs_store::ObjectId) -> String {
    w.export_text(id, MARKDOWN).await.unwrap()
}

#[tokio::test]
async fn move_rewrites_links_in_other_texts() {
    let w = wiki().await;
    let target = w
        .create("wiki/old-page", "# Old\n\ncontent\n")
        .await
        .unwrap();
    let a = w.create("a", "See [[wiki/old-page]] and [[wiki/old-page#Goals]] and [x](wiki/old-page) and [lbl](wiki/old-page \"t\").\n\nShown [[wiki/old-page|label]].\n").await.unwrap();
    let b = w.create("b", "A child [[wiki/old-page/sub]], a lookalike [[wiki/old-page2]], a URL [e](https://e.org/wiki/old-page).\n").await.unwrap();
    let c = w.create("c", "nothing to see\n").await.unwrap();
    let before_b = md(&w, b.id).await;
    let before_c = w.get(c.id).await.unwrap().unwrap();

    let moved = w.move_to(target.id, "archive/new").await.unwrap();

    assert_eq!(moved.path, "archive/new");
    assert_eq!(moved.id, target.id);
    assert_eq!(moved.envelope.iri, target.envelope.iri);
    assert_eq!(
        moved.document.blocks, target.document.blocks,
        "the text itself is untouched"
    );
    assert!(w.get_by_path("wiki/old-page").await.unwrap().is_none());
    assert_eq!(
        w.get_by_path("archive/new").await.unwrap().unwrap().id,
        target.id
    );

    assert_eq!(
        md(&w, a.id).await,
        "See [[archive/new]] and [[archive/new#Goals]] and [x](archive/new) and [lbl](archive/new \"t\").\n\nShown [[archive/new|label]].\n"
    );
    assert_eq!(
        md(&w, b.id).await,
        before_b,
        "children, lookalikes and URLs stay"
    );
    assert_eq!(
        w.get(c.id).await.unwrap().unwrap(),
        before_c,
        "unrelated texts are not even written"
    );

    // The link table follows: back references under the new path, none under the old.
    assert_eq!(w.backlinks("archive/new").await.unwrap().len(), 5);
    assert!(w.backlinks("wiki/old-page").await.unwrap().is_empty());
    let rows = w.links(a.id).await.unwrap();
    assert!(rows.iter().all(|r| r.target.starts_with("archive/new")));
}

#[tokio::test]
async fn links_to_the_moved_text_inside_itself_follow_too() {
    let w = wiki().await;
    let t = w
        .create("old", "I link to [[old]] and [[old#part|myself]].\n")
        .await
        .unwrap();
    w.move_to(t.id, "new/place").await.unwrap();
    assert_eq!(
        md(&w, t.id).await,
        "I link to [[new/place]] and [[new/place#part|myself]].\n"
    );
}

#[tokio::test]
async fn a_taken_or_invalid_path_changes_nothing() {
    let w = wiki().await;
    let a = w.create("a", "[[b]]").await.unwrap();
    let b = w.create("b", "x").await.unwrap();
    let c = w.create("c", "x").await.unwrap();
    assert!(matches!(
        w.move_to(b.id, "a").await,
        Err(Error::PathTaken(_))
    ));
    assert!(matches!(
        w.move_to(b.id, "../x").await,
        Err(Error::InvalidPath { .. })
    ));
    assert!(matches!(
        w.move_to(fs_store::ObjectId::new(), "z").await,
        Err(Error::NotFound(_))
    ));
    assert_eq!(w.get(a.id).await.unwrap().unwrap(), a);
    assert_eq!(w.get(b.id).await.unwrap().unwrap(), b);
    // Moving onto its own path is nothing, and so is the path being free after a move away.
    assert_eq!(w.move_to(c.id, "c").await.unwrap(), c);
    w.move_to(b.id, "b2").await.unwrap();
    assert!(w.create("b", "free again").await.is_ok());
}

#[tokio::test]
async fn a_move_that_stopped_half_way_is_finished_by_repeating_it() {
    // Simulated by hand: "a" already follows the move, the moved text itself is still at "old".
    let w = wiki().await;
    let t = w.create("old", "x").await.unwrap();
    let a = w.create("a", "[[old]]").await.unwrap();
    let b = w.create("b", "[[old]]").await.unwrap();
    let mut doc = a.document.clone();
    fs_text::rewrite_links(std::slice::from_mut(&mut doc), "old", "new");
    w.edit(a.id, doc).await.unwrap();
    w.move_to(t.id, "new").await.unwrap();
    assert_eq!(md(&w, a.id).await, "[[new]]\n");
    assert_eq!(md(&w, b.id).await, "[[new]]\n");
}

#[tokio::test]
async fn a_move_is_undoable_text_by_text() {
    let (w, store) = setup().await;
    let t = w.create("old", "x").await.unwrap();
    let a = w.create("a", "[[old]]").await.unwrap();
    w.move_to(t.id, "new").await.unwrap();
    assert_eq!(md(&w, a.id).await, "[[new]]\n");
    store.undo(ME, 2).await.unwrap(); // the moved text, then the rewrite in "a"
    assert_eq!(w.get(t.id).await.unwrap().unwrap().path, "old");
    assert_eq!(md(&w, a.id).await, "[[old]]\n");
}
