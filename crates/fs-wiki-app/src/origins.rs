//! What a file looked like when it was last imported or exported: the records, in a file of this app.
//!
//! Pattern: Repository over a log file (Memento store) — `Origins` of the core, which is not part of
//! the contract: the records belong to the app and its docs folder, not to the wiki server. The file
//! holds one JSON record per line; a later line for the same file replaces the earlier one, so a save
//! is one short append (synced) and not a rewrite of the whole file. A torn last line (the power went
//! while it was written) is ignored when the file is read. At every start, and after a failed
//! append, the file is written anew under a temporary name and renamed over the old one, so there is
//! always either the old or the new file and it holds each record once. One file per docs folder
//! (`store_path::origins_file`); a second program on the same file is refused at its start by a lock
//! (`flock` on `<file>.lock`). The file and the wiki server are independent: records of one server
//! make no sense on another, which `import.rs` checks.
//!
//! The blocking write and sync run on a thread for blocking work (`spawn_blocking`), so a slow disk
//! never holds a thread of the runtime. `Origins` therefore needs a Tokio runtime to run in.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use fs_wiki_core::{Error, FileOrigin, Origins, Result};

use crate::origins_log::{line, lock, lock_path, parse, rewrite};

/// The records of one docs folder.
pub struct FileOrigins {
    inner: Arc<Inner>,
    /// Holds the lock of the file until this is dropped.
    _lock: File,
}

struct Inner {
    path: PathBuf,
    state: Mutex<State>,
}

struct State {
    records: BTreeMap<String, FileOrigin>,
    /// The file open for appending; `None` after a failed write: the next write starts the file anew.
    log: Option<File>,
}

fn backend(what: &str, path: &Path, e: &dyn std::fmt::Display) -> Error {
    Error::Store(fs_store::Error::Backend(format!(
        "records of imported files, {what} {}: {e}",
        path.display()
    )))
}

impl FileOrigins {
    /// Open the records in `path`, creating the file (and its folder) if there is none.
    ///
    /// # Errors
    /// The file cannot be read or written, a line other than the last does not read (the file is
    /// then left as it is), or another program holds the records of this folder.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| backend("cannot create the folder of", dir, &e))?;
        }
        let held = lock(path).map_err(|e| {
            let busy = e.kind() == io::ErrorKind::WouldBlock;
            let why = if busy {
                "another program is using them (one app per documentation folder)".to_string()
            } else {
                e.to_string()
            };
            backend("cannot lock", &lock_path(path), &why)
        })?;
        let records = match std::fs::read_to_string(path) {
            Ok(text) => parse(&text).map_err(|e| backend("cannot read", path, &e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(backend("cannot read", path, &e)),
        };
        let log = rewrite(path, &records).map_err(|e| backend("cannot write", path, &e))?;
        let state = State {
            records,
            log: Some(log),
        };
        Ok(Self {
            inner: Arc::new(Inner {
                path: path.to_path_buf(),
                state: Mutex::new(state),
            }),
            _lock: held,
        })
    }
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, origin: &FileOrigin) -> Result<()> {
        let mut state = self.state();
        let written = match state.log.as_mut() {
            Some(log) => {
                line(origin).and_then(|l| log.write_all(&l).and_then(|()| log.sync_data()))
            }
            None => Err(io::Error::other("an earlier write failed")),
        };
        let earlier = state.records.insert(origin.file.clone(), origin.clone());
        if written.is_err() {
            // The tail of the file is unknown now: start it anew, with the record included.
            state.log = None;
            match rewrite(&self.path, &state.records) {
                Ok(log) => state.log = Some(log),
                Err(e) => {
                    match earlier {
                        Some(old) => state.records.insert(old.file.clone(), old),
                        None => state.records.remove(&origin.file),
                    };
                    return Err(backend("cannot write", &self.path, &e));
                }
            }
        }
        Ok(())
    }
}

fn joined<T>(r: std::result::Result<Result<T>, tokio::task::JoinError>) -> Result<T> {
    r.unwrap_or_else(|e| Err(Error::Store(fs_store::Error::Backend(e.to_string()))))
}

#[async_trait]
impl Origins for FileOrigins {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        let inner = self.inner.clone();
        let read = tokio::task::spawn_blocking(move || {
            Ok(inner.state().records.values().cloned().collect())
        });
        joined(read.await)
    }

    async fn set_origin(&self, origin: &FileOrigin) -> Result<()> {
        let (inner, origin) = (self.inner.clone(), origin.clone());
        joined(tokio::task::spawn_blocking(move || inner.set(&origin)).await)
    }
}
