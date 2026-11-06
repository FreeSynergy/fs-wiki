//! `move_folder`: the text at a path and everything below it move, links follow. Every test runs
//! on a real in-memory `TursoStore`.

mod common;

use common::{everything, fill, md, paths, setup, ME};
use fs_wiki_core::{Error, Texts};

#[tokio::test]
async fn a_folder_moves_with_everything_below_and_links_follow() {
    let w = common::wiki().await;
    let ids = fill(&w).await;
    let before_old = w.get(ids[4]).await.unwrap().unwrap();

    let moved = w.move_folder("manual", "guide").await.unwrap();
    assert_eq!(paths(&moved), ["guide", "guide/a", "guide/deep/b"]);
    assert_eq!(
        moved.iter().map(|t| t.id).collect::<Vec<_>>(),
        ids[..3].to_vec()
    );

    let all = everything(&w).await;
    assert_eq!(
        paths(&all),
        ["guide", "guide/a", "guide/deep/b", "manual-old", "other"]
    );
    assert_eq!(
        md(&w, ids[0]).await,
        "# Manual\n\nSee [[guide/a]] and [[guide/deep/b#part|b]].\n"
    );
    assert_eq!(
        md(&w, ids[1]).await,
        "Back to [[guide]], up [x](guide/deep/b), self [[guide/a#s]].\n"
    );
    assert_eq!(
        md(&w, ids[2]).await,
        "Link [[guide/a]] and a dangling [[guide/nothing-here]].\n"
    );
    assert_eq!(
        md(&w, ids[3]).await,
        "[[guide]] [[guide#top]] [[guide/a]] [[guide/deep/b#h]] [[manual-old]] [[mine]] [e](https://e.org/manual/a)\n"
    );
    // A lookalike does not move; its link into the folder does follow.
    let old = w.get(ids[4]).await.unwrap().unwrap();
    assert_eq!(old.path, "manual-old");
    assert_eq!(md(&w, ids[4]).await, "Not part of it: [[guide/a]]\n");
    assert_ne!(old.document, before_old.document);

    // The link table agrees: nothing points into the old folder any more.
    assert!(w.backlinks("manual/a").await.unwrap().is_empty());
    assert!(w.backlinks("manual").await.unwrap().is_empty());
    assert_eq!(w.backlinks("guide/a").await.unwrap().len(), 5);
}

#[tokio::test]
async fn a_folder_without_a_text_of_its_own_moves_too() {
    let w = common::wiki().await;
    let a = w.create("docs/x/one", "[[docs/x/two]]").await.unwrap();
    w.create("docs/x/two", "t").await.unwrap();
    let moved = w.move_folder("docs/x", "archive/y").await.unwrap();
    assert_eq!(paths(&moved), ["archive/y/one", "archive/y/two"]);
    assert_eq!(md(&w, a.id).await, "[[archive/y/two]]\n");
    // Moving up into an ancestor works while no name repeats.
    let up = w.move_folder("archive/y", "archive").await.unwrap();
    assert_eq!(paths(&up), ["archive/one", "archive/two"]);
}

#[tokio::test]
async fn a_clash_anywhere_below_refuses_everything_before_writing() {
    let (w, store) = setup().await;
    fill(&w).await;
    w.create("guide/deep/b", "taken").await.unwrap();
    let before = everything(&w).await;
    let log_before = store.undo_state(ME).await.unwrap();

    let err = w.move_folder("manual", "guide").await.unwrap_err();
    assert!(
        matches!(&err, Error::PathTaken(p) if p == "guide/deep/b"),
        "{err:?}"
    );
    assert_eq!(everything(&w).await, before, "no text was touched");
    assert_eq!(
        store.undo_state(ME).await.unwrap(),
        log_before,
        "not one write reached the store"
    );

    // The clash may be the folder's own text, too.
    let w2 = common::wiki().await;
    fill(&w2).await;
    w2.create("guide", "taken").await.unwrap();
    assert!(matches!(
        w2.move_folder("manual", "guide").await,
        Err(Error::PathTaken(p)) if p == "guide"
    ));
}

#[tokio::test]
async fn into_itself_below_itself_and_onto_itself() {
    let (w, store) = setup().await;
    fill(&w).await;
    let state = store.undo_state(ME).await.unwrap();
    for target in ["manual/inside", "manual/deep/er"] {
        assert!(matches!(
            w.move_folder("manual", target).await,
            Err(Error::InvalidMove { .. })
        ));
    }
    // A lookalike name is not "inside".
    assert!(w.move_folder("manual/deep", "manual-deep").await.is_ok());
    w.move_folder("manual-deep", "manual/deep").await.unwrap();
    let state2 = store.undo_state(ME).await.unwrap();
    assert_ne!(state, state2);

    let snapshot = everything(&w).await;
    let same = w.move_folder("manual", "manual").await.unwrap();
    assert_eq!(paths(&same), ["manual", "manual/a", "manual/deep/b"]);
    assert_eq!(everything(&w).await, snapshot);
    assert_eq!(
        store.undo_state(ME).await.unwrap(),
        state2,
        "nothing written"
    );

    assert!(matches!(
        w.move_folder("nothing", "x").await,
        Err(Error::NoSuchFolder(_))
    ));
    assert!(matches!(
        w.move_folder("manual", "../x").await,
        Err(Error::InvalidPath { .. })
    ));
}

#[tokio::test]
async fn moving_up_where_a_name_repeats_is_refused_before_writing() {
    // a/b/b/x -> a/b/x would land inside the old folder a/b: an interrupted move of that kind could
    // not be told from a finished one.
    let w = common::wiki().await;
    w.create("a/b", "1").await.unwrap();
    w.create("a/b/b/x", "2").await.unwrap();
    let before = everything(&w).await;
    assert!(matches!(
        w.move_folder("a/b", "a").await,
        Err(Error::InvalidMove { .. })
    ));
    assert_eq!(everything(&w).await, before);
}
