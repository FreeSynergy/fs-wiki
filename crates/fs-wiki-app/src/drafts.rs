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

/// Drafts as files in one folder, one subfolder per documentation source.
pub struct FileDraftStore {
    dir: PathBuf,
}

/// A name stays readable (the hex of the path) up to this many characters; longer ones are hashed
/// (file names are limited to 255 bytes, and a path of 127 bytes already doubles to that).
const READABLE_NAME: usize = 100;

/// FNV-1a, 64 bit: a hash that is the same on every machine and Rust version.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl FileDraftStore {
    /// A store in `dir` for the documents of `docs` (made when the first draft is saved). Drafts of
    /// two wikis in the same folder do not meet: each documentation source has its own subfolder.
    ///
    /// # Errors
    /// A message when `dir` lies inside `docs`.
    pub fn new(dir: impl Into<PathBuf>, docs: &Path) -> Result<Self, String> {
        let dir = dir.into();
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
        let source = absolute(docs);
        let key = format!("{:016x}", fnv(source.to_string_lossy().as_bytes()));
        Ok(Self { dir: dir.join(key) })
    }

    /// The file of a document: the hex of its path, or a hash for long paths (the path itself is
    /// inside the file).
    fn file(&self, path: &str) -> PathBuf {
        let mut name = String::with_capacity(path.len() * 2 + 6);
        for b in path.bytes() {
            let _ = write!(name, "{b:02x}");
        }
        if name.len() > READABLE_NAME {
            name = format!("h{:016x}", fnv(path.as_bytes()));
        }
        name.push_str(".draft");
        self.dir.join(name)
    }

    /// The folder, private to the user (drafts are private text).
    fn make_dir(&self) -> Result<(), SourceError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| failed(&e))?;
        // Also the folder above it (the drafts base), which `create_dir_all` made too.
        for d in [Some(self.dir.as_path()), self.dir.parent()]
            .into_iter()
            .flatten()
        {
            private(d, 0o700)?;
        }
        Ok(())
    }
}

/// Set the permission bits (Unix); nothing elsewhere.
fn private(path: &Path, mode: u32) -> Result<(), SourceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .map_err(|e| failed(&e))?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
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
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        self.make_dir()?;
        let stored = Stored {
            path: draft.path.clone(),
            text: draft.text.clone(),
            original: draft.original.clone(),
            revision: draft.revision.clone(),
        };
        let json = serde_json::to_vec(&stored).map_err(|e| failed(&e))?;
        let file = self.file(&draft.path);
        let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp = file.with_extension(format!("{}.{unique}.tmp", std::process::id()));
        let written = (|| -> Result<(), SourceError> {
            use std::io::Write as _;
            let mut f = create_private(&temp).map_err(|e| failed(&e))?;
            f.write_all(&json).map_err(|e| failed(&e))?;
            f.sync_all().map_err(|e| failed(&e))?;
            std::fs::rename(&temp, &file).map_err(|e| failed(&e))?;
            // Make the rename itself durable.
            std::fs::File::open(&self.dir)
                .and_then(|d| d.sync_all())
                .map_err(|e| failed(&e))
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        written
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

/// A new file only the user can read.
fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
