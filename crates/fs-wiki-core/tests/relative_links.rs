//! Links written relative to their page: a move changes only the links it really changes.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::{md, wiki};
use fs_wiki_core::Texts;

#[tokio::test]
async fn an_unrelated_move_does_not_touch_a_link_that_only_looks_like_the_moved_folder() {
    let w = wiki().await;
    w.create("a/x.md", "# X\n").await.unwrap();
    // `a/x.md` from `b/y.md` means `b/a/x.md`, which is not the moved page.
    let y = w.create("b/y.md", "[l](a/x.md)\n").await.unwrap();
    w.move_folder("a", "c").await.unwrap();
    assert_eq!(md(&w, y.id).await, "[l](a/x.md)\n");
}

#[tokio::test]
async fn relative_links_to_the_moved_folder_follow_and_stay_relative() {
    let w = wiki().await;
    w.create("a/x.md", "# X\n").await.unwrap();
    let y = w
        .create("b/y.md", "[u](../a/x.md#h) [d](../a/) [w](../a/x.md)\n")
        .await
        .unwrap();
    let z = w
        .create("b/deep/z.md", "[u](../../a/x.md) [dot](./../../a/x.md)\n")
        .await
        .unwrap();
    let top = w
        .create("top.md", "[t](a/x.md) [dot](./a/x.md) [abs](/a/x.md)\n")
        .await
        .unwrap();
    w.move_folder("a", "c").await.unwrap();
    assert_eq!(
        md(&w, y.id).await,
        "[u](../c/x.md#h) [d](../c/) [w](../c/x.md)\n"
    );
    assert_eq!(
        md(&w, z.id).await,
        "[u](../../c/x.md) [dot](../../c/x.md)\n"
    );
    assert_eq!(
        md(&w, top.id).await,
        "[t](c/x.md) [dot](./c/x.md) [abs](/c/x.md)\n"
    );
    let back = w.backlinks("c/x.md").await.unwrap();
    assert_eq!(back.len(), 7);
}

#[tokio::test]
async fn links_inside_a_moved_page_are_written_again_for_its_new_place() {
    let w = wiki().await;
    let b = w
        .create(
            "b/y.md",
            "[o](../other.md) [s](./sib.md) [wiki](other.md)\n[[b/sib.md]]\n",
        )
        .await
        .unwrap();
    w.create("other.md", "o").await.unwrap();
    w.create("b/sib.md", "s").await.unwrap();
    // Alone: one folder deeper.
    w.move_to(b.id, "b/deeper/y.md").await.unwrap();
    assert_eq!(
        md(&w, b.id).await,
        "[o](../../other.md) [s](../sib.md) [wiki](../other.md)\n[[b/sib.md]]\n"
    );
}

#[tokio::test]
async fn a_folder_moved_with_its_neighbours_keeps_their_links_and_fixes_the_ones_leaving() {
    let w = wiki().await;
    let p = w
        .create("a/p.md", "[n](./q.md) [out](../keep.md) [in](sub/r.md)\n")
        .await
        .unwrap();
    w.create("a/q.md", "q").await.unwrap();
    w.create("a/sub/r.md", "r").await.unwrap();
    w.create("keep.md", "k").await.unwrap();
    w.move_folder("a", "x/y/a").await.unwrap();
    assert_eq!(
        md(&w, p.id).await,
        "[n](./q.md) [out](../../../keep.md) [in](sub/r.md)\n"
    );
}

#[tokio::test]
async fn a_link_definition_follows_the_move_too() {
    let w = wiki().await;
    w.create("t/p.md", "p").await.unwrap();
    let q = w
        .create("q.md", "See [r] here.\n\n[r]: t/p.md\n")
        .await
        .unwrap();
    w.move_folder("t", "u").await.unwrap();
    let text = md(&w, q.id).await;
    assert!(text.contains("u/p.md"), "{text:?}");
    assert!(
        !text.contains("t/p.md"),
        "the definition kept the old target: {text:?}"
    );
}

#[tokio::test]
async fn links_without_the_extension_or_to_a_folder_follow_a_page_move() {
    let w = wiki().await;
    let page = w.create("page.md", "p").await.unwrap();
    let idx = w.create("a/index.md", "i").await.unwrap();
    let s = w
        .create("s/src.md", "[p](../page) [a](../a/) [x](../a/index.md)\n")
        .await
        .unwrap();
    w.move_to(page.id, "other/page.md").await.unwrap();
    w.move_to(idx.id, "z/index.md").await.unwrap();
    assert_eq!(
        md(&w, s.id).await,
        "[p](../other/page) [a](../z/) [x](../z/index.md)\n"
    );
    assert_eq!(w.backlinks("other/page.md").await.unwrap().len(), 1);
}
