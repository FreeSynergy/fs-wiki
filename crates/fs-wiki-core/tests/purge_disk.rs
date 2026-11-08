//! After a successful purge the secret is gone from the bytes on disk (database and write-ahead log), not
//! only from the store's queries and log.
//!
//! Pattern: Example-based integration tests over a store in a file, with the stand-in purge port.

mod common;

use std::sync::Arc;

use common::ports::DirectPort;
use common::{doc_with, say};
use fs_objects::{Store, TursoStore};
use fs_wiki_core::{Texts, WikiRepo};

/// All bytes of all files in `dir`, as text.
fn bytes(dir: &std::path::Path) -> String {
    let mut out = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            out.push_str(&String::from_utf8_lossy(&std::fs::read(&p).unwrap()));
        }
    }
    out
}

async fn wiki(dir: &std::path::Path) -> WikiRepo {
    let store = Arc::new(TursoStore::open(dir.join("objects.db")).await.unwrap());
    let as_store: Arc<dyn Store> = store;
    WikiRepo::new(as_store.clone(), common::ME).with_log_purge(Arc::new(DirectPort(as_store)))
}

#[tokio::test]
async fn a_purged_revision_is_not_left_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let w = wiki(dir.path()).await;
    let t = w.create("a", "DISK-REV-SECRET\n").await.unwrap();
    let t = say(&w, &t, "two\n").await;
    let t = say(&w, &t, "three\n").await;
    assert!(
        bytes(dir.path()).contains("DISK-REV-SECRET"),
        "the test sees the bytes before"
    );
    w.purge_revision(t.id, 1).await.unwrap();
    assert!(!bytes(dir.path()).contains("DISK-REV-SECRET"));
    let _ = doc_with(&t, "x\n");
}

#[tokio::test]
async fn a_purged_text_is_not_left_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let w = wiki(dir.path()).await;
    let t = w.create("b", "DISK-TRASH-SECRET\n").await.unwrap();
    let t = say(&w, &t, "DISK-TRASH-SECRET again\n").await;
    w.trash(t.id).await.unwrap();
    assert!(bytes(dir.path()).contains("DISK-TRASH-SECRET"));
    w.purge(t.id).await.unwrap();
    assert!(!bytes(dir.path()).contains("DISK-TRASH-SECRET"));
}
