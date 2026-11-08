//! Listing by path prefix and finding by term: both answered by the store.

mod common;

use common::{paths, wiki};
use fs_wiki_core::{Paging, Texts};

#[tokio::test]
async fn list_is_in_path_order_by_prefix_and_paged() {
    let w = wiki().await;
    for p in ["manual/b", "manual/a", "manual2/x", "other", "manual/sub/c"] {
        w.create(p, "text").await.unwrap();
    }
    let all = w.list("", Paging::default()).await.unwrap();
    assert_eq!(
        paths(&all),
        ["manual/a", "manual/b", "manual/sub/c", "manual2/x", "other"]
    );
    let manual = w.list("manual/", Paging::default()).await.unwrap();
    assert_eq!(paths(&manual), ["manual/a", "manual/b", "manual/sub/c"]);
    assert_eq!(
        paths(&w.list("manual/", Paging::new(1, 1)).await.unwrap()),
        ["manual/b"]
    );
    assert!(
        w.list("Manual/", Paging::default())
            .await
            .unwrap()
            .is_empty(),
        "prefix is case sensitive"
    );
    assert_eq!(
        w.list("nope/", Paging::default()).await.unwrap(),
        [] as [fs_wiki_core::TextEntry; 0]
    );
}

#[tokio::test]
async fn find_looks_at_name_path_and_text_inside_the_store() {
    let w = wiki().await;
    w.create(
        "manual/start",
        "# Welcome\n\nBring a Towel.\n\nSecond paragraph about tea.\n",
    )
    .await
    .unwrap();
    w.create("tea/green", "# Green\n\nnothing\n").await.unwrap();
    w.create("other", "# Other\n\nunrelated\n").await.unwrap();
    let find = |t: &'static str| {
        let w = &w;
        async move {
            paths(&w.find(t, Paging::default()).await.unwrap())
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        }
    };
    assert_eq!(
        find("towel").await,
        ["manual/start"],
        "inside a paragraph, any case"
    );
    assert_eq!(
        find("TEA").await,
        ["manual/start", "tea/green"],
        "text and path"
    );
    assert_eq!(find("welc").await, ["manual/start"], "name");
    assert_eq!(find("zzz").await, [] as [std::string::String; 0]);
    assert_eq!(w.find("tea", Paging::new(1, 1)).await.unwrap().len(), 1);
}
