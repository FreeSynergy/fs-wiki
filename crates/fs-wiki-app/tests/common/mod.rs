//! Helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_store::{Store, TursoStore};
use fs_wiki_core::WikiRepo;
use tokio::runtime::Runtime;

/// A wiki over a fresh in-memory store, and the runtime that drives it.
pub fn wiki() -> (Runtime, Arc<WikiRepo>) {
    let rt = Runtime::new().unwrap();
    let store: Arc<dyn Store> =
        rt.block_on(async { Arc::new(TursoStore::open_in_memory().await.unwrap()) });
    (
        rt,
        Arc::new(WikiRepo::new(store, "https://example.org/actors/t")),
    )
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

/// A wiki over a store file (the file survives `drop`; open it again to "restart").
pub fn wiki_in_file(file: &Path) -> (Runtime, Arc<WikiRepo>) {
    let rt = Runtime::new().unwrap();
    let store: Arc<dyn Store> =
        rt.block_on(async { Arc::new(TursoStore::open(file).await.unwrap()) });
    (
        rt,
        Arc::new(WikiRepo::new(store, "https://example.org/actors/t")),
    )
}

/// The Markdown of the text at `path`, if there is one.
pub fn markdown_at(rt: &Runtime, wiki: &WikiRepo, path: &str) -> Option<String> {
    use fs_wiki_core::{MARKDOWN, Texts};
    rt.block_on(async {
        let entry = wiki.get_by_path(path).await.unwrap()?;
        Some(wiki.export_text(entry.id, MARKDOWN).await.unwrap())
    })
}

/// Save `text` as a person would in the reader.
pub fn save(rt: &Runtime, wiki: &Arc<WikiRepo>, path: &str, text: &str) {
    use fs_components_workbenches::DocumentSource;
    let src = fs_wiki_app::WikiSource::new(wiki.clone(), rt.handle().clone(), vec![]);
    let rev = src.source_text(path).unwrap().revision;
    src.save(path, text, rev.as_deref()).unwrap();
}
