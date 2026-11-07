//! What a file looked like when it was last imported or exported: the records, in a file of this app.
//!
//! Pattern: Repository over a log file (Memento store) — `Origins` of the core, which is not part of
//! the contract: the records belong to the app and its docs folder, not to the wiki server. The file
//! holds one JSON record per line; a later line for the same file replaces the earlier one, so a save
//! is one short append (synced) and not a rewrite of the whole file. A torn last line (the power went
//! while it was written) is ignored when the file is read. At every start, and after a failed
//! append, the file is written anew under a temporary name and renamed over the old one, so there is
//! always either the old or the new file and it holds each record once. One file per docs folder
//! (`store_path::origins_file`). Two programs on one file are not covered.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use fs_wiki_core::{Error, FileOrigin, Origins, Result};

/// The records of one docs folder.
pub struct FileOrigins {
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
    /// The file cannot be read or written, or a line other than the last does not read (the file is
    /// then left as it is).
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| backend("cannot create the folder of", dir, &e))?;
        }
        let records = match std::fs::read_to_string(path) {
            Ok(text) => parse(&text).map_err(|e| backend("cannot read", path, &e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(backend("cannot read", path, &e)),
        };
        let log = rewrite(path, &records).map_err(|e| backend("cannot write", path, &e))?;
        Ok(Self {
            path: path.to_path_buf(),
            state: Mutex::new(State {
                records,
                log: Some(log),
            }),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The records of `text`; the last line may be torn, any other bad line is an error.
fn parse(text: &str) -> io::Result<BTreeMap<String, FileOrigin>> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut records = BTreeMap::new();
    for (i, line) in lines.iter().enumerate() {
        match serde_json::from_str::<FileOrigin>(line) {
            Ok(r) => {
                records.insert(r.file.clone(), r);
            }
            Err(_) if i + 1 == lines.len() && !text.ends_with('\n') => {}
            Err(e) => {
                let why = format!("line {} does not read: {e}", i + 1);
                return Err(io::Error::new(io::ErrorKind::InvalidData, why));
            }
        }
    }
    Ok(records)
}

fn line(record: &FileOrigin) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(record).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write all `records` to a temporary file, sync it, rename it over `path`, sync the folder, and
/// open `path` for appending.
fn rewrite(path: &Path, records: &BTreeMap<String, FileOrigin>) -> io::Result<File> {
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    for record in records.values() {
        file.write_all(&line(record)?)?;
    }
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        File::open(dir)?.sync_all()?;
    }
    OpenOptions::new().append(true).open(path)
}

#[async_trait]
impl Origins for FileOrigins {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        Ok(self.state().records.values().cloned().collect())
    }

    async fn set_origin(&self, origin: &FileOrigin) -> Result<()> {
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
