//! `links` and `backlinks` through the client, and the limit of the wire (`LinkRow`).
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{make, Served};
use fs_wiki_core::Texts;

#[tokio::test(flavor = "multi_thread")]
async fn links_are_the_rows_into_the_wiki_as_written_with_no_title() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(
        &texts,
        "a",
        "See [[b#part|bee]], [c](c.md) and [out](https://example.org/x).\n",
    )
    .await;
    let rows = texts.links(a.id).await.unwrap();
    let targets: Vec<_> = rows.iter().map(|r| r.target.as_str()).collect();
    assert_eq!(targets, ["b#part", "c.md"], "a URL is no row on the wire");
    assert!(rows.iter().all(|r| r.source == a.id && r.internal));
    assert!(
        rows.iter().all(|r| r.title.is_empty()),
        "no title on the wire"
    );
    assert!(
        rows[0].at.contains(':') && rows[0].at.contains('-'),
        "{}",
        rows[0].at
    );
    // What the core knows and the wire does not: the same text read inside the server.
    let inside = served.repo.links(a.id).await.unwrap();
    assert!(
        inside.iter().any(|l| !l.internal),
        "the core still has the URL"
    );
    assert!(inside.iter().any(|l| l.title == "bee"));
}

#[tokio::test(flavor = "multi_thread")]
async fn backlinks_find_the_texts_that_point_at_a_path() {
    let served = Served::start().await;
    let texts = served.client();
    let x = make(&texts, "x", "to [[target]]\n").await;
    let y = make(&texts, "y", "to [[target#h]] and [[elsewhere]]\n").await;
    make(&texts, "z", "nothing\n").await;
    let rows = texts.backlinks("target").await.unwrap();
    let mut got: Vec<_> = rows.iter().map(|r| (r.source, r.target.clone())).collect();
    got.sort();
    let mut want = vec![(x.id, "target".to_string()), (y.id, "target#h".to_string())];
    want.sort();
    assert_eq!(got, want);
    assert!(rows.iter().all(|r| r.internal));
}
