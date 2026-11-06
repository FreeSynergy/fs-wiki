//! Compare-and-set edits: the revision counts writes, a stale writer gets a named conflict.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use std::sync::Arc;

use common::{md, wiki};
use fs_text::Document;
use fs_wiki_core::{Error, TextFormat, Texts};

#[tokio::test]
async fn the_revision_counts_every_write_and_a_stale_edit_is_refused() {
    let w = wiki().await;
    let t = w.create("p", "one\n").await.unwrap();
    assert_eq!(t.revision, 1);
    let mut doc = t.document.clone();
    assert!(TextFormat::Markdown.import_external(&mut doc, "two\n"));
    let t2 = w.edit_if(t.id, 1, doc).await.unwrap();
    assert_eq!(t2.revision, 2);

    let mut stale = t.document.clone();
    TextFormat::Markdown.import_external(&mut stale, "three\n");
    let err = w.edit_if(t.id, 1, stale).await.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Conflict {
                expected: 1,
                found: 2,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(md(&w, t.id).await, "two\n", "nothing was written");

    // A move is a write too.
    let moved = w.move_to(t.id, "q").await.unwrap();
    assert_eq!(moved.revision, 3);
}

#[tokio::test]
async fn racing_writers_never_lose_an_accepted_update() {
    let w = Arc::new(wiki().await);
    let t = w.create("p", "start\n").await.unwrap();
    let mut tasks = vec![];
    for n in 0..4 {
        let w = w.clone();
        tasks.push(tokio::spawn(async move {
            let mut accepted = vec![];
            for i in 0..50 {
                let word = format!("w{n}x{i}");
                let cur = w.get(t.id).await.unwrap().unwrap();
                let mut doc: Document = cur.document.clone();
                let text = format!(
                    "{} {word}\n",
                    TextFormat::Markdown.export(&cur.document).trim()
                );
                TextFormat::Markdown.import_external(&mut doc, &text);
                if w.edit_if(t.id, cur.revision, doc).await.is_ok() {
                    accepted.push(word);
                }
            }
            accepted
        }));
    }
    let mut words = vec![];
    for t in tasks {
        words.extend(t.await.unwrap());
    }
    let last = w.get(t.id).await.unwrap().unwrap();
    assert_eq!(
        last.revision,
        1 + words.len() as u64,
        "one revision per accepted edit"
    );
    let text = TextFormat::Markdown.export(&last.document);
    let lost: Vec<_> = words
        .iter()
        .filter(|x| !text.contains(x.as_str()))
        .collect();
    assert!(lost.is_empty(), "accepted but lost: {lost:?}");
}
