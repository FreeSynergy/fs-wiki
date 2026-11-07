//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_objects::{Store, TursoStore};
use fs_wiki_core::{TextEntry, WikiRepo};

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
