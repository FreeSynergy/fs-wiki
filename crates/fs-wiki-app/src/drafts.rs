//! Backup of unsaved drafts in a folder of the app's own data directory.
//!
//! Pattern: Adapter — `FileDraftStore` implements the reader's `DraftStore` with one small JSON
//! file per document. The write is atomic (temporary file, flush to disk, rename), so a crash
//! leaves the old backup or the new one. The folder must not be inside the documentation folder
//! (the docs are only changed by the explicit Export, ADR 0042) and a backup is never written
//! back by itself (ADR 0046).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use fs_components_workbenches::{Draft, DraftStore, SourceError};
use serde::{Deserialize, Serialize};

/// The default folder: `$XDG_DATA_HOME/fs-wiki/drafts`, else `~/.local/share/fs-wiki/drafts`.
pub fn default_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("fs-wiki").join("drafts"))
}

/// The file form of a draft.
#[derive(Serialize, Deserialize)]
struct Stored {
    path: String,
    text: String,
    original: String,
    revision: Option<String>,
}

/// Drafts as files in one folder.
pub struct FileDraftStore {
    dir: PathBuf,
}

impl FileDraftStore {
    /// A store in `dir` (made when the first draft is saved). `docs` is the documentation folder:
    /// a store inside it is refused.
    ///
    /// # Errors
    /// A message when `dir` lies inside `docs`.
    pub fn new(dir: PathBuf, docs: &Path) -> Result<Self, String> {
        let inside = |a: &Path, b: &Path| {
            let (a, b) = (absolute(a), absolute(b));
            a.starts_with(&b)
        };
        if inside(&dir, docs) {
            return Err(format!(
                "the drafts folder {} is inside the documentation folder {}; drafts must not go there",
                dir.display(),
                docs.display()
            ));
        }
        Ok(Self { dir })
    }

    /// The file of a document: the hex of its path, so every path is a safe name.
    fn file(&self, path: &str) -> PathBuf {
        let mut name = String::with_capacity(path.len() * 2 + 6);
        for b in path.bytes() {
            let _ = write!(name, "{b:02x}");
        }
        name.push_str(".draft");
        self.dir.join(name)
    }
}

/// An absolute form without touching the disk for names that do not exist yet.
fn absolute(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| {
        let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
        // Canonicalise the nearest existing parent so that links in the way count.
        let mut tail = Vec::new();
        let mut at = abs.as_path();
        while let Some(parent) = at.parent() {
            if let Ok(real) = std::fs::canonicalize(at) {
                return tail.iter().rev().fold(real, |acc, part| acc.join(part));
            }
            if let Some(name) = at.file_name() {
                tail.push(name.to_owned());
            }
            at = parent;
        }
        abs
    })
}

fn failed(e: &dyn std::fmt::Display) -> SourceError {
    SourceError::failed(e.to_string())
}

impl DraftStore for FileDraftStore {
    fn save(&self, draft: &Draft) -> Result<(), SourceError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| failed(&e))?;
        let stored = Stored {
            path: draft.path.clone(),
            text: draft.text.clone(),
            original: draft.original.clone(),
            revision: draft.revision.clone(),
        };
        let json = serde_json::to_vec(&stored).map_err(|e| failed(&e))?;
        let file = self.file(&draft.path);
        let temp = file.with_extension("tmp");
        {
            use std::io::Write as _;
            let mut f = std::fs::File::create(&temp).map_err(|e| failed(&e))?;
            f.write_all(&json).map_err(|e| failed(&e))?;
            f.sync_all().map_err(|e| failed(&e))?;
        }
        std::fs::rename(&temp, &file).map_err(|e| failed(&e))
    }

    fn load(&self, path: &str) -> Result<Option<Draft>, SourceError> {
        let file = self.file(path);
        let bytes = match std::fs::read(&file) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(failed(&e)),
        };
        let s: Stored = serde_json::from_slice(&bytes).map_err(|e| {
            SourceError::failed(format!("the backup {} is damaged: {e}", file.display()))
        })?;
        Ok(Some(Draft::new(s.path, s.text, s.original, s.revision)))
    }

    fn clear(&self, path: &str) -> Result<(), SourceError> {
        match std::fs::remove_file(self.file(path)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(failed(&e)),
        }
    }
}
