//! Helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

mod served;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_store::{Store, TursoStore};
use fs_wiki_app::{FileOrigins, Wiki};
use fs_wiki_client::RpcTexts;
use served::Served;
use tokio::runtime::Runtime;

/// The wiki of a test: the app's `Wiki` (texts over a real socket, records in a real file) and the
/// real server it talks to, which stays up as long as this lives.
pub struct Fixture {
    // Dropped first: the connection closes before the server stops.
    wiki: Arc<Wiki>,
    _served: Served,
    /// The folder of the records of a wiki that has no store file of its own (see `wiki`).
    _records: Option<tempfile::TempDir>,
}

impl Fixture {
    /// The wiki to hand to the app's code (`import_folder`, `WikiSource`).
    pub fn wiki(&self) -> Arc<Wiki> {
        self.wiki.clone()
    }
}

impl Deref for Fixture {
    type Target = Wiki;
    fn deref(&self) -> &Wiki {
        &self.wiki
    }
}

fn fixture(
    store: Arc<dyn Store>,
    records: &Path,
    folder: Option<tempfile::TempDir>,
) -> Arc<Fixture> {
    let served = Served::over(store);
    let texts = RpcTexts::connect(&served.socket).unwrap();
    let origins = FileOrigins::open(records).unwrap();
    let wiki = Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)));
    Arc::new(Fixture {
        wiki,
        _served: served,
        _records: folder,
    })
}

/// A wiki over a fresh in-memory store, and the runtime that drives it. The records of imported
/// files are in a folder of their own, removed with the test.
pub fn wiki() -> (Runtime, Arc<Fixture>) {
    let rt = Runtime::new().unwrap();
    let store: Arc<dyn Store> =
        rt.block_on(async { Arc::new(TursoStore::open_in_memory().await.unwrap()) });
    let folder = tempfile::tempdir().unwrap();
    let records = folder.path().join("wiki.origins");
    (rt, fixture(store, &records, Some(folder)))
}

/// The folders a test made; removed when the test's thread ends, also when it panics.
struct Cleanup(Vec<PathBuf>);

impl Drop for Cleanup {
    fn drop(&mut self) {
        for dir in &self.0 {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

thread_local! {
    static CLEANUP: std::cell::RefCell<Cleanup> = const { std::cell::RefCell::new(Cleanup(Vec::new())) };
}

/// A fresh empty folder under the system temp dir, removed when the test ends.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fs-wiki-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    CLEANUP.with(|c| c.borrow_mut().0.push(dir.clone()));
    dir
}

/// Write `files` (relative path, content) below `root`.
pub fn write_files(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
}

/// Every file below `root` with its bytes and modification time, to prove nothing changed.
pub fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let meta = e.metadata().unwrap();
                out.push((
                    p.strip_prefix(root).unwrap().display().to_string(),
                    std::fs::read(&p).unwrap(),
                    meta.modified().unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

/// A wiki over a store file and a records file next to it (both survive `drop`; open them again
/// to "restart": the server is stopped, a new one is started over the same files).
pub fn wiki_in_file(file: &Path) -> (Runtime, Arc<Fixture>) {
    let rt = Runtime::new().unwrap();
    let store: Arc<dyn Store> =
        rt.block_on(async { Arc::new(TursoStore::open(file).await.unwrap()) });
    (rt, fixture(store, &file.with_extension("origins"), None))
}

/// The Markdown of the text at `path`, if there is one.
pub fn markdown_at(rt: &Runtime, wiki: &Wiki, path: &str) -> Option<String> {
    use fs_wiki_core::{MARKDOWN, Texts};
    rt.block_on(async {
        let entry = wiki.get_by_path(path).await.unwrap()?;
        Some(wiki.export_text(entry.id, MARKDOWN).await.unwrap())
    })
}

/// Save `text` as a person would in the reader.
pub fn save(rt: &Runtime, wiki: &Fixture, path: &str, text: &str) {
    use fs_components_workbenches::DocumentSource;
    let src = fs_wiki_app::WikiSource::new(wiki.wiki(), rt.handle().clone(), vec![]);
    let rev = src.source_text(path).unwrap().revision;
    src.save(path, text, rev.as_deref()).unwrap();
}
