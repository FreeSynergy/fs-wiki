//! Create, read, edit, list and delete texts through the `Texts` trait over a real `TursoStore`.

mod common;

use common::{setup, wiki, ME};
use fs_store::ObjectId;
use fs_text::{Block, Mark, MarkKind, MarkValue};
use fs_wiki_core::{Error, Paging, TextPath, Texts};

const START: &str = "# Getting started\n\nWelcome to the **manual**.\n";

#[tokio::test]
async fn a_text_has_the_article_envelope_and_a_derived_name() {
    let w = wiki().await.with_instance("wiki.example.org");
    let t = w.create("manual/start", START).await.unwrap();
    assert_eq!(t.envelope.object_type, "Article");
    assert_eq!(t.envelope.context.as_deref(), Some("texts"));
    assert_eq!(
        t.envelope.iri.as_deref(),
        Some(format!("https://wiki.example.org/texts/{}", t.id).as_str())
    );
    assert_eq!(t.envelope.attributed_to.as_deref(), Some(ME));
    assert_eq!(
        t.envelope.name, "Getting started",
        "ADR 0032: from the first heading"
    );
    assert_eq!(t.path, "manual/start");
    assert_eq!(t.document.id, t.id.as_uuid());
    assert_eq!(w.get(t.id).await.unwrap().unwrap(), t);
    assert_eq!(w.get_by_path("manual/start").await.unwrap().unwrap(), t);
    assert!(w.get(ObjectId::new()).await.unwrap().is_none());
    assert!(w.get_by_path("manual").await.unwrap().is_none());
}

#[tokio::test]
async fn the_name_falls_back_to_front_matter_text_and_path() {
    let w = wiki().await;
    let fm = w
        .create("a", "---\ntitle: \"From front matter\"\n---\n\n# Heading\n")
        .await
        .unwrap();
    assert_eq!(fm.envelope.name, "From front matter");
    let para = w
        .create("b", "just some words\nsecond line\n")
        .await
        .unwrap();
    assert_eq!(para.envelope.name, "just some words");
    let empty = w.create("deep/er/last", "").await.unwrap();
    assert_eq!(empty.envelope.name, "last");
}

#[tokio::test]
async fn a_path_is_unique_and_must_be_readable() {
    let w = wiki().await;
    w.create("manual/start", START).await.unwrap();
    assert!(matches!(
        w.create("manual/start", "other").await,
        Err(Error::PathTaken(p)) if p == "manual/start"
    ));
    assert!(w.create("Manual/start", "case counts").await.is_ok());
    for bad in [
        "", "/a", "a/", "a//b", "a/../b", "./a", "a#b", "a\\b", "a/ b", "a\nb",
    ] {
        assert!(
            matches!(w.create(bad, "x").await, Err(Error::InvalidPath { .. })),
            "{bad:?}"
        );
    }
    assert_eq!(TextPath::parse("a/b").unwrap().last_segment(), "b");
    assert_eq!(
        w.list("", Paging::default()).await.unwrap().len(),
        2,
        "nothing bad was stored"
    );
}

#[tokio::test]
async fn edit_replaces_the_document_and_keeps_the_envelope() {
    let w = wiki().await;
    let old = w.create("p", START).await.unwrap();
    let mut doc = old.document.clone();
    doc.blocks.insert(
        0,
        Block::new(fs_text::BlockKind::Heading { level: 1 }, "Renamed"),
    );
    doc.id = fs_text::id::new_id(); // forced back to the text's id
    let new = w.edit(old.id, doc).await.unwrap();
    assert_eq!(new.id, old.id);
    assert_eq!(new.document.id, old.id.as_uuid());
    assert_eq!(new.envelope.name, "Renamed");
    assert_eq!(new.envelope.published, old.envelope.published);
    assert_eq!(new.envelope.iri, old.envelope.iri);
    assert_eq!(new.path, "p");
    assert!(matches!(
        w.edit(ObjectId::new(), new.document).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn layers_are_stored_with_the_document() {
    let w = wiki().await;
    let t = w
        .create("p", "Remember this warning well.\n")
        .await
        .unwrap();
    let mut doc = t.document.clone();
    doc.blocks[0].add_mark(Mark::new(
        MarkKind::Passage,
        0,
        8,
        MarkValue::Name("note".into()),
    ));
    doc.blocks[0].add_mark(Mark::point(
        MarkKind::Variable,
        9,
        MarkValue::Variable {
            name: "who".into(),
            ty: None,
        },
    ));
    w.edit(t.id, doc.clone()).await.unwrap();
    let back = w.get(t.id).await.unwrap().unwrap();
    assert_eq!(
        back.document.blocks[0].marks.as_slice(),
        doc.blocks[0].marks.as_slice()
    );
    assert_eq!(back.document.blocks[0].marks.len(), 2);
}

#[tokio::test]
async fn delete_takes_the_text_with_its_layers_and_is_undoable() {
    let (w, store) = setup().await;
    let t = w.create("p", START).await.unwrap();
    w.create("q", "keep").await.unwrap();
    w.delete(t.id).await.unwrap();
    assert!(w.get(t.id).await.unwrap().is_none());
    assert!(w.get_by_path("p").await.unwrap().is_none());
    assert!(matches!(w.delete(t.id).await, Err(Error::NotFound(_))));
    assert!(w.create("p", "the path is free again").await.is_ok());
    store.undo(ME, 1).await.unwrap(); // undoes the create of "p" again
    store.undo(ME, 1).await.unwrap(); // and the delete
    assert_eq!(w.get(t.id).await.unwrap().unwrap().path, "p");
}
