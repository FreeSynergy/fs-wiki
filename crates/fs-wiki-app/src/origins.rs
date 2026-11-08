//! What a file looked like when it was last imported or exported: the records, in a file of this app.
//!
//! Pattern: Repository over a log file — `Origins` of the core, which is not part of
//! the contract: the records belong to the app and its docs folder, not to the wiki server. The file
//! holds one JSON record per line; a later line for the same file replaces the earlier one, so a save
//! is one short append (synced) and not a rewrite of the whole file. A torn last line (the power went
//! while it was written) is ignored when the file is read. At every start, and after a failed
//! append, the file is written anew under a temporary name and renamed over the old one, so there is
//! always either the old or the new file and it holds each record once. One file per docs folder
//! (`store_path::origins_file`); a second program on the same file is refused at its start by a lock
//! (`flock` on `<file>.lock`) that is held until the last write has ended. The records and the wiki
//! server are independent: records of one server make no sense on another, which `import.rs` checks.
//!
//! The blocking write and sync run on a thread for blocking work (`spawn_blocking`), so a slow disk
//! never holds a thread of the runtime. `Origins` therefore needs a Tokio runtime to run in.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use fs_objects::StoreId;
use fs_wiki_core::{Error, FileOrigin, Foreign, Origins, Result};

use crate::origins_log::{Book, Records, line, lock, lock_path, parse, rewrite};
use crate::words::{self, say};

/// Why the records could not be opened. Each is a kind of its own; the sentence is the `Display`.
#[derive(Debug)]
pub enum OriginsError {
    /// Another program holds the records of this folder.
    Busy {
        /// The lock file.
        lock: PathBuf,
    },
    /// A line other than the last does not read; the file was left as it is.
    Damaged {
        /// The records file.
        path: PathBuf,
        /// The line (from 1).
        line: usize,
        /// The parser's reason.
        why: String,
    },
    /// The system refused: a whole sentence that says what and why.
    Failed(String),
}

impl std::fmt::Display for OriginsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy { lock } => {
                f.write_str(&say(words::RECORDS_BUSY, &[("path", &lock.display())]))
            }
            Self::Damaged { path, line, why } => f.write_str(&say(
                words::RECORDS_DAMAGED,
                &[("path", &path.display()), ("line", line), ("why", why)],
            )),
            Self::Failed(sentence) => f.write_str(sentence),
        }
    }
}

impl std::error::Error for OriginsError {}

impl From<OriginsError> for Error {
    fn from(e: OriginsError) -> Self {
        Self::Store(fs_objects::Error::Backend(e.to_string()))
    }
}

/// The records of one docs folder.
pub struct FileOrigins {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    /// The lock of the records file: it lives as long as the last job that may still write.
    _lock: File,
}

struct State {
    book: Book,
    /// The store the caller is connected to; `None` before `belong_to` or for a provider without an
    /// identity: then the records are those from before stores were recorded.
    current: Option<StoreId>,
    /// The store whose records the end of the file holds; appending is only right for `current`.
    tail: Option<StoreId>,
    /// The file open for appending; `None` after a failed write: the next write starts the file anew.
    log: Option<File>,
}

impl State {
    fn records(&self) -> &Records {
        match self.current.and_then(|s| self.book.stores.get(&s)) {
            Some(records) => records,
            None => &self.book.legacy,
        }
    }

    fn records_mut(&mut self) -> &mut Records {
        match self.current {
            Some(store) => self.book.stores.entry(store).or_default(),
            None => &mut self.book.legacy,
        }
    }
}

fn failed(sentence: &str, path: &Path, why: &dyn std::fmt::Display) -> OriginsError {
    OriginsError::Failed(say(sentence, &[("path", &path.display()), ("why", why)]))
}

impl FileOrigins {
    /// Open the records in `path`, creating the file (and its folder) if there is none.
    ///
    /// # Errors
    /// [`OriginsError`]: the file cannot be read or written, a line other than the last does not
    /// read, or another program holds the records of this folder.
    pub fn open(path: &Path) -> std::result::Result<Self, OriginsError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| failed(words::RECORDS_FOLDER, dir, &e))?;
        }
        let held = lock(path).map_err(|e| match e.kind() {
            io::ErrorKind::WouldBlock => OriginsError::Busy {
                lock: lock_path(path),
            },
            _ => failed(words::RECORDS_LOCK, &lock_path(path), &e),
        })?;
        let book = match std::fs::read_to_string(path) {
            Ok(text) => parse(&text).map_err(|(line, why)| OriginsError::Damaged {
                path: path.to_path_buf(),
                line,
                why,
            })?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Book::default(),
            Err(e) => return Err(failed(words::RECORDS_READ, path, &e)),
        };
        let (log, tail) =
            rewrite(path, &book, None).map_err(|e| failed(words::RECORDS_WRITE, path, &e))?;
        let state = State {
            book,
            current: None,
            tail,
            log: Some(log),
        };
        Ok(Self {
            inner: Arc::new(Inner {
                path: path.to_path_buf(),
                state: Mutex::new(state),
                _lock: held,
            }),
        })
    }
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, origin: &FileOrigin) -> Result<()> {
        let mut state = self.state();
        // Appending is right only at the end of the records of `current`; else the file is
        // written anew with them last.
        let appendable =
            state.tail == state.current && (state.current.is_some() || state.tail.is_none());
        let written = match state.log.as_mut() {
            Some(log) if appendable => {
                line(origin).and_then(|l| log.write_all(&l).and_then(|()| log.sync_data()))
            }
            _ => Err(io::Error::other(
                "the end of the file is not these records'",
            )),
        };
        let earlier = state
            .records_mut()
            .insert(origin.file.clone(), origin.clone());
        if written.is_err() {
            // The tail of the file is unknown now: start it anew, with the record included.
            state.log = None;
            match rewrite(&self.path, &state.book, state.current) {
                Ok((log, tail)) => {
                    state.log = Some(log);
                    state.tail = tail;
                }
                Err(e) => {
                    match earlier {
                        Some(old) => state.records_mut().insert(old.file.clone(), old),
                        None => state.records_mut().remove(&origin.file),
                    };
                    return Err(failed(words::RECORDS_WRITE, &self.path, &e).into());
                }
            }
        }
        Ok(())
    }

    fn belong_to(&self, store: Option<StoreId>) -> Foreign {
        let mut state = self.state();
        state.current = store;
        if let Some(store) = store {
            state.book.stores.entry(store).or_default();
        }
        let others = state
            .book
            .stores
            .iter()
            .filter(|(s, _)| Some(**s) != store)
            .map(|(_, r)| r.len())
            .sum();
        let legacy = match store {
            Some(_) => state.book.legacy.values().cloned().collect(),
            None => Vec::new(),
        };
        Foreign { legacy, others }
    }

    fn adopt_legacy(&self) -> Result<()> {
        let mut state = self.state();
        let Some(store) = state.current else {
            return Ok(());
        };
        let mut book = state.book.clone();
        let legacy = std::mem::take(&mut book.legacy);
        let section = book.stores.entry(store).or_default();
        for (file, record) in legacy {
            section.entry(file).or_insert(record);
        }
        // The file is written first: a failure leaves memory as it was.
        let (log, tail) = rewrite(&self.path, &book, Some(store))
            .map_err(|e| failed(words::RECORDS_WRITE, &self.path, &e))?;
        state.book = book;
        state.log = Some(log);
        state.tail = tail;
        Ok(())
    }
}

fn joined<T>(r: std::result::Result<Result<T>, tokio::task::JoinError>) -> Result<T> {
    r.unwrap_or_else(|e| Err(Error::Store(fs_objects::Error::Backend(e.to_string()))))
}

#[async_trait]
impl Origins for FileOrigins {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        let inner = self.inner.clone();
        let read = tokio::task::spawn_blocking(move || {
            Ok(inner.state().records().values().cloned().collect())
        });
        joined(read.await)
    }

    async fn set_origin(&self, origin: &FileOrigin) -> Result<()> {
        let (inner, origin) = (self.inner.clone(), origin.clone());
        joined(tokio::task::spawn_blocking(move || inner.set(&origin)).await)
    }

    async fn belong_to(&self, store: Option<StoreId>) -> Result<Foreign> {
        Ok(self.inner.belong_to(store))
    }

    async fn adopt_legacy(&self) -> Result<()> {
        let inner = self.inner.clone();
        joined(tokio::task::spawn_blocking(move || inner.adopt_legacy()).await)
    }
}
