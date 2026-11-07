//! The problems of the server come back as the errors of the core, and the connection stays.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{make, paths, Served};
use fs_store::ObjectId;
use fs_wiki_core::{Error, Paging, TextFormat, Texts};
use fs_wiki_server::MAX_SOURCE_BYTES;

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_text_is_not_found_with_its_id() {
    let served = Served::start().await;
    let texts = served.client();
    let id = ObjectId::new();
    let doc = TextFormat::Markdown.import("x\n");
    let e = texts.edit(id, doc.clone()).await.unwrap_err();
    assert!(matches!(e, Error::NotFound(i) if i == id), "{e:?}");
    let e = texts.edit_if(id, 1, doc).await.unwrap_err();
    assert!(matches!(e, Error::NotFound(i) if i == id), "{e:?}");
    let e = texts.links(id).await.unwrap_err();
    assert!(matches!(e, Error::NotFound(i) if i == id), "{e:?}");
    let e = texts.export_text(id, "text/markdown").await.unwrap_err();
    assert!(matches!(e, Error::NotFound(i) if i == id), "{e:?}");
    let e = texts.move_to(id, "x").await.unwrap_err();
    assert!(matches!(e, Error::NotFound(i) if i == id), "{e:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stale_revision_is_a_conflict_with_the_numbers_and_writes_nothing() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(&texts, "a", "one\n").await;
    let newer = texts
        .edit(a.id, TextFormat::Markdown.import("two\n"))
        .await
        .unwrap();
    let doc = TextFormat::Markdown.import("three\n");
    let e = texts.edit_if(a.id, a.revision, doc).await.unwrap_err();
    match e {
        Error::Conflict {
            id,
            expected,
            found,
        } => {
            assert_eq!((id, expected, found), (a.id, a.revision, newer.revision));
        }
        other => panic!("{other:?}"),
    }
    let now = texts.get(a.id).await.unwrap().unwrap();
    assert_eq!(TextFormat::Markdown.export(&now.document), "two\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_taken_path_and_a_bad_path_are_told_apart() {
    let served = Served::start().await;
    let texts = served.client();
    make(&texts, "a", "x\n").await;
    let e = texts.create("a", "y\n").await.unwrap_err();
    assert!(matches!(&e, Error::PathTaken(p) if p == "a"), "{e:?}");
    for bad in ["a//b", "../x", "a#b", "/lead", "tab\there"] {
        let e = texts.create(bad, "y\n").await.unwrap_err();
        assert!(
            matches!(&e, Error::InvalidPath { path, .. } if path == bad),
            "{bad:?}: {e:?}"
        );
    }
    assert_eq!(paths(&texts, "").await, ["a"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_server_cannot_do_is_unsupported_with_its_capability() {
    let served = Served::start().await;
    let texts = served.client();
    let a = make(&texts, "a", "x\n").await;
    let e = texts.export_text(a.id, "text/html").await.unwrap_err();
    assert!(
        matches!(&e, Error::Unsupported(u) if u.capability() == "text.export"),
        "{e:?}"
    );
    let e = texts
        .import_text("b", "text/html", "<p>")
        .await
        .unwrap_err();
    assert!(
        matches!(&e, Error::Unsupported(u) if u.capability() == "text.import"),
        "{e:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_request_the_server_refuses_as_invalid_or_too_large_is_an_error_not_a_hang() {
    let served = Served::start().await;
    let texts = served.client();
    let e = texts.find("", Paging::default()).await.unwrap_err();
    assert!(
        matches!(&e, Error::Store(fs_store::Error::Invalid(t)) if t.contains("term")),
        "{e:?}"
    );
    let big = "x".repeat(MAX_SOURCE_BYTES + 1);
    let e = texts.create("big", &big).await.unwrap_err();
    assert!(
        matches!(&e, Error::Store(fs_store::Error::Invalid(t)) if t.contains("too large")),
        "{e:?}"
    );
    assert!(
        texts.create("fine", "small\n").await.is_ok(),
        "the connection stays"
    );
}
