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
