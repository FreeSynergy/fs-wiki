//! Comments written as link definitions (`[//]: # (note)`) are not content of the page.
//!
//! Pattern: Example-based integration test over a real in-memory store.

mod common;

use fs_wiki_core::{Paging, Texts};

#[tokio::test]
async fn a_private_comment_is_not_found_by_search() {
    let w = common::wiki().await;
    w.create(
        "p",
        "A visible-word here.\n\n[//]: # (private comment)\n\n[//]: <> (second private)\n",
    )
    .await
    .unwrap();
    assert_eq!(
        w.find("visible-word", Paging::default())
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(w
        .find("private", Paging::default())
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn every_comment_form_is_not_displayed_but_stays_in_the_text() {
    let w = common::wiki().await;
    let t = w
        .create(
            "p",
            "Shown words.\n\n[comment]: # (hush-one)\n\n<!-- hush-two -->\n\n[//]: <> (hush-three)\n\n[ref]: https://example.org/x \"a title\"\n",
        )
        .await
        .unwrap();
    assert_eq!(w.find("Shown", Paging::default()).await.unwrap().len(), 1);
    for word in ["hush-one", "hush-two", "hush-three"] {
        assert!(
            w.find(word, Paging::default()).await.unwrap().is_empty(),
            "{word}"
        );
    }
    // A real definition is content, and nothing is deleted: the export keeps every comment.
    assert_eq!(
        w.find("example.org", Paging::default())
            .await
            .unwrap()
            .len(),
        1
    );
    let text = w.export_text(t.id, "text/markdown").await.unwrap();
    for word in ["hush-one", "hush-two", "hush-three"] {
        assert!(text.contains(word), "{word}");
    }
}
