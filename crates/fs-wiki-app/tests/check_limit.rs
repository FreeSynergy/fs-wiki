//! The comparison of the records with the server is bounded: a server that does not answer ends the
//! start with a failure, not a wait of hours.
//!
//! Pattern: Integration test against the real server. The only stand-in is the provider in front of
//! it, which delegates to the real wiki and holds the server thread on a `get`: a stuck server.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use common::{temp_dir, write_files};
use fs_objects::ObjectId;
use fs_wiki_app::{FileOrigins, Wiki, import_folder_within};
use fs_wiki_client::RpcTexts;
use fs_wiki_core::{Capabilities, FileOrigin, Origins, Paging, Result, TextEntry, Texts, WikiRepo};
use fs_wiki_testserver::Served;

/// The real wiki whose first `get` does not answer for a long time.
struct Stuck(Arc<WikiRepo>, AtomicBool);

#[async_trait]
impl Texts for Stuck {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        if !self.1.swap(true, Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(4));
        }
        self.0.get(id).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.0.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.0.list(prefix, page).await
    }
}

#[test]
fn a_server_that_does_not_answer_ends_the_start_within_the_limit() {
    let dir = temp_dir("cl-stuck");
    let docs = dir.join("docs");
    write_files(&docs, &[("a.md", "# A\n")]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let plain = rt.block_on(Served::in_memory());
    let served = Served::serving(
        Arc::new(Stuck(plain.repo.clone(), AtomicBool::new(false))),
        plain.repo.clone(),
    );
    let origins = FileOrigins::open(&dir.join("o.origins")).unwrap();
    let record = FileOrigin {
        file: "a.md".into(),
        hash: "h".into(),
        text: ObjectId::new(),
        text_hash: "t".into(),
        deleted: false,
    };
    rt.block_on(origins.set_origin(&record)).unwrap();
    let texts = RpcTexts::connect(&served.socket).unwrap();
    let wiki = Wiki::new(Arc::new(texts), Arc::new(origins));
    let started = Instant::now();
    let report = rt.block_on(import_folder_within(
        &wiki,
        &docs,
        Duration::from_millis(300),
    ));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert_eq!((report.failures.len(), report.imported), (1, 0));
    std::fs::remove_dir_all(dir).unwrap();
}
