//! `move_to` and `move_folder`: the texts that move, the links that follow, the refusals.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{make, paths, ClientOf, Served};
use fs_wiki_core::{Error, TextFormat, Texts};

fn markdown(t: &fs_wiki_core::TextEntry) -> String {
    TextFormat::Markdown.export(&t.document)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_moved_text_keeps_its_id_and_the_links_to_it_follow() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let a = make(&texts, "a", "# A\n").await;
    let b = make(&texts, "b", "see [[a]]\n").await;
    let moved = texts.move_to(a.id, "z/a").await.unwrap();
    assert_eq!((moved.id, moved.path.as_str()), (a.id, "z/a"));
    let b = texts.get(b.id).await.unwrap().unwrap();
    assert!(markdown(&b).contains("z/a"), "{}", markdown(&b));
    assert_eq!(paths(&texts, "").await, ["b", "z/a"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_moves_with_everything_below_it() {
    let served = Served::in_memory().await;
    let texts = served.client();
    make(&texts, "old", "# Index\n").await;
    make(&texts, "old/x", "x\n").await;
    make(&texts, "old/sub/y", "y\n").await;
    make(&texts, "keep", "[[old/x]]\n").await;
    let moved = texts.move_folder("old", "new").await.unwrap();
    let mut got: Vec<_> = moved.iter().map(|t| t.path.as_str()).collect();
    got.sort_unstable();
    assert_eq!(got, ["new", "new/sub/y", "new/x"]);
    assert_eq!(
        paths(&texts, "").await,
        ["keep", "new", "new/sub/y", "new/x"]
    );
    let keep = texts.get_by_path("keep").await.unwrap().unwrap();
    assert!(markdown(&keep).contains("new/x"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_refusals_of_a_move_are_the_errors_of_the_core() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let a = make(&texts, "a", "x\n").await;
    make(&texts, "b", "y\n").await;
    make(&texts, "f/one", "1\n").await;
    make(&texts, "g/two", "2\n").await;
    let e = texts.move_to(a.id, "b").await.unwrap_err();
    assert!(matches!(&e, Error::PathTaken(p) if p == "b"), "{e:?}");
    let e = texts.move_to(a.id, "a//b").await.unwrap_err();
    assert!(
        matches!(&e, Error::InvalidPath { path, .. } if path == "a//b"),
        "{e:?}"
    );
    let e = texts.move_folder("nothing", "x").await.unwrap_err();
    assert!(
        matches!(&e, Error::NoSuchFolder(p) if p == "nothing"),
        "{e:?}"
    );
    let e = texts.move_folder("f", "g").await.unwrap_err();
    assert!(
        matches!(&e, Error::TargetNotEmpty { from, to } if from == "f" && to == "g"),
        "{e:?}"
    );
    let e = texts.move_folder("f", "f/inside").await.unwrap_err();
    assert!(
        matches!(&e, Error::InvalidMove { from, to, .. } if from == "f" && to == "f/inside"),
        "{e:?}"
    );
    make(&texts, "p/q/q/x", "3\n").await;
    make(&texts, "p/q/z", "4\n").await;
    let e = texts.move_folder("p/q", "p").await.unwrap_err();
    assert!(matches!(&e, Error::InvalidMove { .. }), "{e:?}");
    assert_eq!(paths(&texts, "f/").await, ["f/one"], "nothing was moved");
}
