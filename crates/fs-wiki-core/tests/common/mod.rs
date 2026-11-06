//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_store::{Store, TursoStore};
use fs_wiki_core::{Paging, TextEntry, Texts, WikiRepo, MARKDOWN};

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
pub async fn fill(w: &WikiRepo) -> Vec<fs_store::ObjectId> {
    let mut ids = vec![];
    for (path, md) in [
        ("manual", "# Manual\n\nSee [[manual/a]] and [[manual/deep/b#part|b]].\n"),
        ("manual/a", "Back to [[manual]], up [x](manual/deep/b), self [[manual/a#s]].\n"),
        ("manual/deep/b", "Link [[manual/a]] and a dangling [[manual/nothing-here]].\n"),
        ("other", "[[manual]] [[manual#top]] [[manual/a]] [[manual/deep/b#h]] [[manual-old]] [[mine]] [e](https://e.org/manual/a)\n"),
        ("manual-old", "Not part of it: [[manual/a]]\n"),
    ] {
        ids.push(w.create(path, md).await.unwrap().id);
    }
    ids
}

pub async fn md(w: &WikiRepo, id: fs_store::ObjectId) -> String {
    w.export_text(id, MARKDOWN).await.unwrap()
}

pub async fn everything(w: &WikiRepo) -> Vec<TextEntry> {
    w.list("", Paging::default()).await.unwrap()
}
