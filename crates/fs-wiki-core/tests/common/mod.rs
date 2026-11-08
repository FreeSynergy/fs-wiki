//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

pub mod counting;
pub mod revfail;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{
    Activity, Error as StoreError, LogEntry, ObjectId, Result as StoreResult, Store, StoredObject,
    TursoStore,
};
use fs_text::Document;
use fs_wiki_core::{Paging, TextEntry, TextFormat, Texts, WikiRepo, MARKDOWN};

pub const ME: &str = "https://example.org/actors/a";

/// A wiki over a fresh in-memory store.
pub async fn wiki() -> WikiRepo {
    setup().await.0
}

/// The wiki and a handle to its store (for undo checks).
pub async fn setup() -> (WikiRepo, Arc<dyn Store>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    (WikiRepo::new(store.clone(), ME), store)
}

pub fn paths(texts: &[TextEntry]) -> Vec<&str> {
    texts.iter().map(|t| t.path.as_str()).collect()
}

/// The standard wiki: a folder `manual` with a text of its own, children, a lookalike, and texts
/// elsewhere that link in. Returns the ids of `[manual, manual/a, manual/deep/b, other, lookalike]`.
pub async fn fill(w: &WikiRepo) -> Vec<fs_objects::ObjectId> {
    let mut ids = vec![];
    for (path, md) in [
        ("manual", "# Manual\n\nSee [[manual/a]] and [[manual/deep/b#part|b]].\n"),
        ("manual/a", "Back to [[manual]], up [x](deep/b), self [[manual/a#s]].\n"),
        ("manual/deep/b", "Link [[manual/a]] and a dangling [[manual/nothing-here]].\n"),
        ("other", "[[manual]] [[manual#top]] [[manual/a]] [[manual/deep/b#h]] [[manual-old]] [[mine]] [e](https://e.org/manual/a)\n"),
        ("manual-old", "Not part of it: [[manual/a]]\n"),
    ] {
        ids.push(w.create(path, md).await.unwrap().id);
    }
    ids
}

pub async fn md(w: &WikiRepo, id: fs_objects::ObjectId) -> String {
    w.export_text(id, MARKDOWN).await.unwrap()
}

pub async fn everything(w: &WikiRepo) -> Vec<TextEntry> {
    w.list("", Paging::default()).await.unwrap()
}

/// A store that stops accepting writes after `left` of them (reads always work).
pub struct Failing {
    pub inner: Arc<TursoStore>,
    pub left: AtomicUsize,
}

impl Failing {
    fn gate(&self) -> StoreResult<()> {
        let left = self.left.load(Ordering::SeqCst);
        if left == 0 {
            return Err(StoreError::Backend("injected failure".into()));
        }
        self.left.store(left - 1, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl ObjectRepository for Failing {
    async fn put(&self, object: StoredObject, actor: &str) -> StoreResult<Activity> {
        // The failure injection counts the writes of texts; the revision of a write is its bookkeeping
        // (texts 0.2), so a test written for "n writes" still means n writes of texts.
        if object.envelope.object_type != "WikiRevision" {
            self.gate()?;
        }
        ObjectRepository::put(&*self.inner, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> StoreResult<Vec<Activity>> {
        self.gate()?;
        ObjectRepository::put_and_log(&*self.inner, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> StoreResult<Option<StoredObject>> {
        ObjectRepository::get(&*self.inner, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> StoreResult<Activity> {
        self.gate()?;
        ObjectRepository::delete(&*self.inner, id, actor).await
    }
}

/// A wiki over `store` whose writes stop after `left`.
pub fn failing(store: &Arc<TursoStore>, left: usize) -> WikiRepo {
    let objects = Arc::new(Failing {
        inner: store.clone(),
        left: AtomicUsize::new(left),
    });
    let reads: Arc<dyn ObjectQuery> = store.clone();
    WikiRepo::with_backend(objects, reads, ME)
}

/// The document of `t` changed to say `markdown` (layers stay).
pub fn doc_with(t: &TextEntry, markdown: &str) -> Document {
    let mut doc = t.document.clone();
    TextFormat::Markdown.import_external(&mut doc, markdown);
    doc
}

/// Edit `t` to say `markdown`.
pub async fn say(w: &WikiRepo, t: &TextEntry, markdown: &str) -> TextEntry {
    w.edit(t.id, doc_with(t, markdown)).await.unwrap()
}
