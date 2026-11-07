//! Shared set-up of the tests: the real server (`fs-wiki-testserver`) and the client connected to it.
//! Nothing here stands in for either.
//!
//! Pattern: Facade over the set-up.
#![allow(dead_code)] // each test file uses a different subset

use fs_wiki_client::RpcTexts;
use fs_wiki_core::{TextEntry, Texts};

pub use fs_wiki_testserver::Served;

/// A client of a served wiki.
pub trait ClientOf {
    /// Connect to this server.
    fn client(&self) -> RpcTexts;
}

impl ClientOf for Served {
    fn client(&self) -> RpcTexts {
        RpcTexts::connect(&self.socket).unwrap()
    }
}

/// Create a text through the client.
pub async fn make(texts: &dyn Texts, path: &str, markdown: &str) -> TextEntry {
    texts.create(path, markdown).await.unwrap()
}

/// The paths of all texts below `prefix`, as the client lists them.
pub async fn paths(texts: &dyn Texts, prefix: &str) -> Vec<String> {
    let all = texts
        .list(prefix, fs_wiki_core::Paging::default())
        .await
        .unwrap();
    all.into_iter().map(|t| t.path).collect()
}
