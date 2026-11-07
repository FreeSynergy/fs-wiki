//! Helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_objects::{Store, TursoStore};
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

/// A fresh empty folder under the system temp dir.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fs-wiki-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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
