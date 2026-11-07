//! `Paging` against the cursor pages of the server, with more hits than one page holds.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::Served;
use fs_interfaces::rpc::DEFAULT_PAGE_SIZE;
use fs_wiki_core::{Paging, Texts};

/// More texts than one page of the server.
const MANY: usize = DEFAULT_PAGE_SIZE * 2 + 30;

async fn filled(texts: &dyn Texts) -> Vec<String> {
    let mut want = Vec::new();
    for i in 0..MANY {
        let path = format!("p/{i:04}");
        texts.create(&path, "needle\n").await.unwrap();
        want.push(path);
    }
    want
}

fn path_of(t: fs_wiki_core::TextEntry) -> String {
    t.path
}

#[tokio::test(flavor = "multi_thread")]
async fn everything_is_listed_over_several_pages_in_order_and_once() {
    let served = Served::start().await;
    let texts = served.client();
    let want = filled(&texts).await;
    let got: Vec<_> = texts
        .list("p/", Paging::default())
        .await
        .unwrap()
        .into_iter()
        .map(path_of)
        .collect();
    assert_eq!(got, want);
}

#[tokio::test(flavor = "multi_thread")]
async fn limit_and_offset_cut_across_the_page_borders() {
    let served = Served::start().await;
    let texts = served.client();
    let want = filled(&texts).await;
    let at = |offset: usize, limit: usize| {
        let texts = &texts;
        async move {
            let page = texts.list("p/", Paging::new(limit, offset)).await.unwrap();
            page.into_iter().map(path_of).collect::<Vec<_>>()
        }
    };
    assert_eq!(at(0, 5).await, want[..5]);
    let border = DEFAULT_PAGE_SIZE - 3;
    assert_eq!(at(border, 10).await, want[border..border + 10]);
    assert_eq!(at(MANY - 4, 50).await, want[MANY - 4..]);
    assert!(at(MANY + 10, 5).await.is_empty());
    assert!(at(0, 0).await.is_empty());
    let only_offset = Paging {
        limit: None,
        offset: MANY - 2,
    };
    let rest = texts.list("p/", only_offset).await.unwrap();
    assert_eq!(rest.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn find_pages_the_same_way() {
    let served = Served::start().await;
    let texts = served.client();
    let want = filled(&texts).await;
    let all = texts.find("needle", Paging::default()).await.unwrap();
    assert_eq!(all.len(), MANY);
    let mut got: Vec<_> = all.into_iter().map(path_of).collect();
    got.sort();
    assert_eq!(got, want);
    let tail = texts
        .find("needle", Paging::new(1000, DEFAULT_PAGE_SIZE + 1))
        .await
        .unwrap();
    assert_eq!(tail.len(), MANY - DEFAULT_PAGE_SIZE - 1);
}
