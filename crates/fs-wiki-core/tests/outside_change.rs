//! Files are exports, the database is the truth (ADR 0042): `importText` on an existing path is
//! a diff that becomes a normal edit; layers move with the text; deleted text takes its marks.

mod common;

use common::{setup, wiki, ME};
use fs_text::{Mark, MarkKind, MarkValue};
use fs_wiki_core::{Error, Texts, MARKDOWN};

const V1: &str = "# Manual\n\nFirst paragraph with a warning inside.\n\nSecond paragraph stays.\n\nThird paragraph goes away.\n";

/// V1 with the second paragraph edited and the third removed.
const V2: &str = "# Manual\n\nFirst paragraph with a warning inside.\n\nSecond paragraph was edited elsewhere.\n";

async fn with_layers(w: &fs_wiki_core::WikiRepo) -> fs_wiki_core::TextEntry {
    let t = w.create("manual", V1).await.unwrap();
    let mut doc = t.document.clone();
    // A named passage over "warning" (first paragraph), an explanation over "Second", and one
    // over "Third paragraph" (which the outside edit will delete).
    let find = |d: &fs_text::Document, s: &str| {
        let i = d
            .blocks
            .iter()
            .position(|b| b.text.to_string().contains(s))
            .unwrap();
        let at = b_text(&d.blocks[i]).find(s).unwrap();
        (
            i,
            b_text(&d.blocks[i])[..at].chars().count(),
            s.chars().count(),
        )
    };
    for (word, kind, value) in [
        ("warning", MarkKind::Passage, MarkValue::Name("warn".into())),
        (
            "Second",
            MarkKind::Explanation,
            MarkValue::Explanation {
                replacement: "mine".into(),
            },
        ),
        (
            "Third paragraph",
            MarkKind::Passage,
            MarkValue::Name("gone".into()),
        ),
    ] {
        let (i, start, len) = find(&doc, word);
        doc.blocks[i].add_mark(Mark::new(kind, start, start + len, value));
    }
    w.edit(t.id, doc).await.unwrap()
}

fn b_text(b: &fs_text::Block) -> String {
    b.text.to_string()
}

fn names(t: &fs_wiki_core::TextEntry) -> Vec<String> {
    t.document
        .blocks
        .iter()
        .flat_map(|b| {
            b.marks.iter().map(|m| match &m.value {
                MarkValue::Name(n) => n.clone(),
                MarkValue::Explanation { replacement } => replacement.clone(),
                _ => String::new(),
            })
        })
        .collect()
}

#[tokio::test]
async fn an_outside_change_is_a_normal_edit_and_layers_stay() {
    let w = wiki().await;
    let before = with_layers(&w).await;
    let after = w.import_text("manual", "text/markdown", V2).await.unwrap();

    assert_eq!(after.id, before.id, "the same text, not a new one");
    assert_eq!(w.export_text(after.id, MARKDOWN).await.unwrap(), V2);
    // Blocks that did not change keep their id, and so their layers.
    assert_eq!(after.document.blocks[1].id, before.document.blocks[1].id);
    assert_eq!(
        after.document.blocks[1].marks.as_slice(),
        before.document.blocks[1].marks.as_slice()
    );
    // The edited paragraph keeps the explanation over the words that are still there ...
    let second = after
        .document
        .blocks
        .iter()
        .find(|b| b.text.to_string().starts_with("Second"))
        .unwrap();
    assert!(second.marks.iter().any(|m| m.kind == MarkKind::Explanation));
    // ... and the paragraph that was deleted takes its passage along.
    assert_eq!(names(&after).iter().filter(|n| *n == "gone").count(), 0);
    assert_eq!(names(&after).iter().filter(|n| *n == "warn").count(), 1);
    assert!(after.envelope.updated >= before.envelope.updated);
    assert_eq!(after.envelope.published, before.envelope.published);
}

#[tokio::test]
async fn layers_are_not_written_into_the_exported_file() {
    let w = wiki().await;
    let t = with_layers(&w).await;
    assert_eq!(names(&t).len(), 3);
    assert_eq!(
        w.export_text(t.id, "text/markdown; charset=utf-8")
            .await
            .unwrap(),
        V1
    );
}

#[tokio::test]
async fn importing_the_same_file_changes_nothing() {
    let w = wiki().await;
    let t = with_layers(&w).await;
    let exported = w.export_text(t.id, MARKDOWN).await.unwrap();
    let again = w.import_text("manual", MARKDOWN, &exported).await.unwrap();
    assert_eq!(again, t, "not even a write: `updated` is the same");
}

#[tokio::test]
async fn a_new_path_is_created_and_a_text_diff_in_one_block_moves_marks() {
    let w = wiki().await;
    let t = w
        .import_text("notes/one", MARKDOWN, "Hello brave world\n")
        .await
        .unwrap();
    assert_eq!(t.path, "notes/one");
    let mut doc = t.document.clone();
    doc.blocks[0].add_mark(Mark::new(
        MarkKind::Passage,
        12,
        17,
        MarkValue::Name("world".into()),
    ));
    w.edit(t.id, doc).await.unwrap();
    // An outside editor inserted a word before it.
    let t2 = w
        .import_text("notes/one", MARKDOWN, "Hello very brave world\n")
        .await
        .unwrap();
    let m = t2.document.blocks[0].marks.as_slice()[0].clone();
    assert_eq!(t2.document.blocks[0].text.slice(m.start..m.end), "world");
    // And one that deleted exactly the marked word: the mark goes with it.
    let t3 = w
        .import_text("notes/one", MARKDOWN, "Hello very brave\n")
        .await
        .unwrap();
    assert!(t3.document.blocks[0].marks.is_empty());
}

#[tokio::test]
async fn the_outside_change_can_be_undone_like_any_edit() {
    let (w, store) = setup().await;
    let t = with_layers(&w).await;
    w.import_text("manual", MARKDOWN, V2).await.unwrap();
    store.undo(ME, 1).await.unwrap();
    assert_eq!(
        w.get(t.id).await.unwrap().unwrap(),
        t,
        "text and layers are back"
    );
}

#[tokio::test]
async fn an_unknown_media_type_is_an_error_and_stores_nothing() {
    let w = wiki().await;
    let t = w.create("p", "x").await.unwrap();
    for bad in ["text/plain", "application/json", "", "text/markdownish"] {
        assert!(
            matches!(w.import_text("q", bad, "x").await, Err(Error::MediaType(_))),
            "{bad:?}"
        );
        assert!(
            matches!(w.export_text(t.id, bad).await, Err(Error::MediaType(_))),
            "{bad:?}"
        );
    }
    assert!(w.get_by_path("q").await.unwrap().is_none());
    assert!(matches!(
        w.import_text("../x", MARKDOWN, "x").await,
        Err(Error::InvalidPath { .. })
    ));
    assert!(matches!(
        w.export_text(fs_objects::ObjectId::new(), MARKDOWN).await,
        Err(Error::NotFound(_))
    ));
}
