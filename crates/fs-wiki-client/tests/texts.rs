//! The reading and writing methods of `Texts`, through the client and the real server.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{make, paths, Served};
use fs_wiki_core::{Paging, TextFormat, Texts, MARKDOWN};

#[tokio::test(flavor = "multi_thread")]
async fn a_text_is_created_read_by_id_and_by_path_and_listed_in_path_order() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(&texts, "manual/start", "# Start\n\nHello.\n").await;
    make(&texts, "manual/end", "# End\n").await;
    make(&texts, "other", "x\n").await;
    assert_eq!(a.path, "manual/start");
    assert_eq!(a.envelope.name, "Start");
    assert_ne!(a.revision, 0);
    let by_id = texts.get(a.id).await.unwrap().unwrap();
    assert_eq!(by_id, a);
    let by_path = texts.get_by_path("manual/start").await.unwrap().unwrap();
    assert_eq!(by_path.id, a.id);
    assert!(texts.get_by_path("nothing").await.unwrap().is_none());
    assert!(texts
        .get(fs_store::ObjectId::new())
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        paths(&texts, "").await,
        ["manual/end", "manual/start", "other"]
    );
    assert_eq!(
        paths(&texts, "manual/").await,
        ["manual/end", "manual/start"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_and_edit_if_replace_the_document_and_change_the_revision() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(&texts, "a", "one\n").await;
    let doc = TextFormat::Markdown.import("two\n");
    let edited = texts.edit(a.id, doc.clone()).await.unwrap();
    assert_ne!(edited.revision, a.revision);
    assert_eq!(TextFormat::Markdown.export(&edited.document), "two\n");
    let doc3 = TextFormat::Markdown.import("three\n");
    let third = texts.edit_if(a.id, edited.revision, doc3).await.unwrap();
    assert_eq!(TextFormat::Markdown.export(&third.document), "three\n");
    let again = texts.get(a.id).await.unwrap().unwrap();
    assert_eq!(again.revision, third.revision);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_removes_the_text() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(&texts, "a", "x\n").await;
    texts.delete(a.id).await.unwrap();
    assert!(texts.get(a.id).await.unwrap().is_none());
    assert!(paths(&texts, "").await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn find_looks_in_name_path_and_text_ignoring_case() {
    let served = Served::start().await;
    let texts = served.client();
    make(&texts, "guides/kanban", "# Boards\n\nplain\n").await;
    make(&texts, "b", "# Other\n\nA KANBAN word\n").await;
    make(&texts, "c", "# Third\n\nnothing\n").await;
    let hits = texts.find("kanban", Paging::default()).await.unwrap();
    let mut got: Vec<_> = hits.into_iter().map(|t| t.path).collect();
    got.sort();
    assert_eq!(got, ["b", "guides/kanban"]);
    let first = texts.find("kanban", Paging::new(1, 0)).await.unwrap();
    assert_eq!(first.len(), 1);
    let second = texts.find("kanban", Paging::new(1, 1)).await.unwrap();
    assert_eq!(second.len(), 1);
    assert_ne!(first[0].id, second[0].id);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_creates_then_updates_and_export_writes_markdown() {
    let served = Served::start().await;
    let texts = served.client();
    let made = texts
        .import_text("file", MARKDOWN, "# T\n\none\n")
        .await
        .unwrap();
    let again = texts
        .import_text("file", MARKDOWN, "# T\n\ntwo\n")
        .await
        .unwrap();
    assert_eq!(
        made.id, again.id,
        "an outside change is an edit, not a new text"
    );
    let md = texts.export_text(again.id, MARKDOWN).await.unwrap();
    assert_eq!(md, "# T\n\ntwo\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_report_of_the_server_is_the_report_of_the_client() {
    let served = Served::start().await;
    let texts = served.client();
    let caps = texts.capabilities();
    for id in [
        "text.read",
        "text.create",
        "text.edit",
        "text.move",
        "text.subscribe",
    ] {
        assert!(
            caps.entries()
                .iter()
                .any(|e| e.capability == id && e.supported),
            "{id}"
        );
    }
}
