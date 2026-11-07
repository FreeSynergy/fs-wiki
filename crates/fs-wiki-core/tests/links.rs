//! The link table: links of one text and back references to a path.

mod common;

use common::wiki;
use fs_objects::ObjectId;
use fs_text::{Mark, MarkKind, MarkValue};
use fs_wiki_core::{Error, Texts};

const PAGE: &str = "See [[manual/start]] and [[manual/start#Goals|the goals]] and [x](manual/other) and [ext](https://example.org/manual/start).\n\nAn image ![logo](img/logo.png) and an [anchor](#top).\n";

#[tokio::test]
async fn links_lists_every_link_with_its_place() {
    let w = wiki().await;
    let t = w.create("home", PAGE).await.unwrap();
    let rows = w.links(t.id).await.unwrap();
    let targets: Vec<_> = rows
        .iter()
        .map(|r| (r.target.as_str(), r.internal))
        .collect();
    assert_eq!(
        targets,
        [
            ("manual/start", true),
            ("manual/start#Goals", true),
            ("manual/other", true),
            ("https://example.org/manual/start", false),
            ("img/logo.png", true),
            ("#top", false),
        ]
    );
    assert!(rows.iter().all(|r| r.source == t.id));
    assert_eq!(rows[1].title, "the goals");
    let block = t.document.blocks[0].id;
    assert!(
        rows[0].at.starts_with(&format!("{block}:")),
        "{}",
        rows[0].at
    );
    assert!(matches!(
        w.links(ObjectId::new()).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn backlinks_find_internal_links_from_all_texts() {
    let w = wiki().await;
    let a = w.create("home", PAGE).await.unwrap();
    let b = w
        .create("other", "Back to [[manual/start]]!\n")
        .await
        .unwrap();
    w.create(
        "noise",
        "[[manual/start-not]] and [x](https://e.org/manual/start)\n",
    )
    .await
    .unwrap();
    let rows = w.backlinks("manual/start").await.unwrap();
    let mut got: Vec<_> = rows.iter().map(|r| (r.source, r.target.as_str())).collect();
    got.sort();
    let mut want = vec![
        (a.id, "manual/start"),
        (a.id, "manual/start#Goals"),
        (b.id, "manual/start"),
    ];
    want.sort();
    assert_eq!(got, want);
    assert_eq!(
        w.backlinks("manual/start#Goals").await.unwrap().len(),
        3,
        "the page part counts"
    );
    assert!(w.backlinks("nobody").await.unwrap().is_empty());
}

#[tokio::test]
async fn backlinks_follow_edits_and_deletes() {
    let w = wiki().await;
    let b = w
        .create("other", "Back to [[manual/start]]!\n")
        .await
        .unwrap();
    assert_eq!(w.backlinks("manual/start").await.unwrap().len(), 1);
    let mut doc = b.document.clone();
    assert!(doc.blocks[0].marks.iter().any(|m| m.kind == MarkKind::Link));
    doc.blocks[0].marks = fs_text::MarkSet::from_vec(vec![]);
    doc.blocks[0].add_mark(Mark::new(MarkKind::Strong, 0, 4, MarkValue::None));
    w.edit(b.id, doc).await.unwrap();
    assert!(
        w.backlinks("manual/start").await.unwrap().is_empty(),
        "the link mark is gone"
    );
    let c = w.create("third", "[[manual/start]]").await.unwrap();
    assert_eq!(w.backlinks("manual/start").await.unwrap().len(), 1);
    w.delete(c.id).await.unwrap();
    assert!(w.backlinks("manual/start").await.unwrap().is_empty());
}
