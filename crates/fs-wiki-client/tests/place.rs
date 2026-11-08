//! `page` and `identity` over the socket give what the provider behind it gives.
//!
//! Pattern: Integration tests against the trait; no stand-ins. The client cannot make the cursor of
//! the server from a place, so it reads from the start; the answers must still be the provider's own.

mod common;

use common::{ClientOf, Served};
use fs_interfaces::rpc::DEFAULT_PAGE_SIZE;
use fs_wiki_core::{TextQuery, Texts};

#[tokio::test(flavor = "multi_thread")]
async fn the_identity_is_the_one_of_the_store_behind_the_server() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let behind = served.repo.identity();
    assert!(behind.is_some());
    assert_eq!(texts.identity(), behind);
}

#[tokio::test(flavor = "multi_thread")]
async fn pages_after_a_place_are_the_ones_of_the_provider_across_server_pages() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let many = DEFAULT_PAGE_SIZE + 25;
    for i in 0..many {
        texts
            .create(&format!("p/{i:04}"), "needle\n")
            .await
            .unwrap();
    }
    for query in [
        TextQuery::Prefix("p/".into()),
        TextQuery::Term("needle".into()),
    ] {
        let (mut mine, mut theirs) = (Vec::new(), Vec::new());
        let (mut a, mut b) = (None, None);
        loop {
            let left = texts.page(&query, a.as_ref(), 40).await.unwrap();
            let right = served.repo.page(&query, b.as_ref(), 40).await.unwrap();
            assert_eq!(left.more, right.more);
            assert_eq!(
                left.items.iter().map(|p| &p.place).collect::<Vec<_>>(),
                right.items.iter().map(|p| &p.place).collect::<Vec<_>>(),
            );
            mine.extend(left.items.iter().map(|p| p.entry.path.clone()));
            theirs.extend(right.items.iter().map(|p| p.entry.path.clone()));
            a = left.items.last().map(|p| p.place.clone());
            b = right.items.last().map(|p| p.place.clone());
            if !left.more {
                break;
            }
        }
        assert_eq!(mine.len(), many);
        assert_eq!(mine, theirs);
    }
}
