//! Link reference definitions: only real ones follow a move, wherever they are written.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::{md, wiki};
use fs_wiki_core::Texts;

#[tokio::test]
async fn text_inside_html_blocks_and_comments_is_not_a_definition() {
    let w = wiki().await;
    w.create("t/p.md", "p").await.unwrap();
    for (name, body) in [
        ("pre", "<pre>\n[r]: t/p.md\n</pre>\n"),
        ("comment", "<!--\n[r]: t/p.md\n-->\n"),
    ] {
        let q = w
            .create(&format!("q-{name}.md"), &format!("[l](t/p.md)\n\n{body}"))
            .await
            .unwrap();
        w.move_folder("t", "u").await.unwrap();
        let text = md(&w, q.id).await;
        assert!(
            text.contains(body),
            "{name}: text in {name} was changed: {text:?}"
        );
        assert!(text.contains("[l](u/p.md)"), "{name}: {text:?}");
        w.move_folder("u", "t").await.unwrap();
    }
}

#[tokio::test]
async fn an_unused_definition_a_wrapped_one_and_a_target_with_a_space() {
    let w = wiki().await;
    w.create("t/p.md", "p").await.unwrap();
    let unused = w.create("q1.md", "[unused]: t/p.md\n").await.unwrap();
    let wrapped = w
        .create("q2.md", "Use [r].\n\n[r]:\n  t/p.md\n")
        .await
        .unwrap();
    w.move_folder("t", "u").await.unwrap();
    assert!(md(&w, unused.id).await.contains("[unused]: u/p.md"));
    let text = md(&w, wrapped.id).await;
    assert!(
        text.contains("u/p.md") && !text.contains("t/p.md"),
        "{text:?}"
    );
    w.move_folder("u", "my dir").await.unwrap();
    assert!(
        md(&w, unused.id).await.contains("[unused]: <my dir/p.md>"),
        "{:?}",
        md(&w, unused.id).await
    );
}
