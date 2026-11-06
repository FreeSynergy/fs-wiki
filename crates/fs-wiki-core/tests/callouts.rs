//! Links in callout titles and second definitions follow a move.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::{md, wiki};
use fs_wiki_core::Texts;

#[tokio::test]
async fn a_link_only_in_a_callout_title_is_found_and_follows_the_move() {
    let w = wiki().await;
    w.create("t/p.md", "p").await.unwrap();
    let only = w
        .create(
            "a/q.md",
            "> [!note] See [x](../t/p.md) and [[t/p.md|P]]\n> body words\n",
        )
        .await
        .unwrap();
    assert_eq!(
        w.backlinks("t/p.md").await.unwrap().len(),
        2,
        "found before the move"
    );
    w.move_folder("t", "u").await.unwrap();
    let text = md(&w, only.id).await;
    assert!(
        text.contains("[x](../u/p.md)") && text.contains("[[u/p.md|P]]"),
        "{text:?}"
    );
    assert!(!text.contains("t/p.md"), "{text:?}");
    // The page itself moves: its relative title link is written again.
    w.move_to(only.id, "a/b/q.md").await.unwrap();
    let text = md(&w, only.id).await;
    assert!(text.contains("[x](../../u/p.md)"), "{text:?}");
}

#[tokio::test]
async fn a_second_definition_with_the_same_label_follows_too() {
    let w = wiki().await;
    w.create("t/p.md", "p").await.unwrap();
    let q = w
        .create("q.md", "Use [r].\n\n[r]: t/p.md\n\n[r]: t/p.md#other\n")
        .await
        .unwrap();
    w.move_folder("t", "u").await.unwrap();
    let text = md(&w, q.id).await;
    assert!(!text.contains("t/p.md"), "{text:?}");
    assert_eq!(text.matches("u/p.md").count(), 3, "{text:?}");
}
