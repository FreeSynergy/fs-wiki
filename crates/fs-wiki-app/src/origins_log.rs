//! The file format of the records: JSON lines, read, rewritten atomically, locked.
//!
//! Pattern: none — free functions on a file.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use fs_objects::StoreId;
use fs_wiki_core::FileOrigin;
use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;
use serde::{Deserialize, Serialize};

/// The line that says which store the records are for (the first line of the file, if any).
#[derive(Serialize, Deserialize)]
struct Header {
    store: StoreId,
}

/// What a records file holds: the records, and the store they were made for if the file says it.
#[derive(Default)]
pub(crate) struct Content {
    pub(crate) records: BTreeMap<String, FileOrigin>,
    pub(crate) store: Option<StoreId>,
}

/// The records of `text`; the last line may be torn, any other bad line is an error: its number and
/// the parser's reason. A file from before the store id has no header line.
pub(crate) fn parse(text: &str) -> Result<Content, (usize, String)> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut content = Content::default();
    for (i, line) in lines.iter().enumerate() {
        match serde_json::from_str::<FileOrigin>(line) {
            Ok(r) => {
                content.records.insert(r.file.clone(), r);
            }
            Err(e) => match serde_json::from_str::<Header>(line) {
                Ok(header) => content.store = Some(header.store),
                Err(_) if i + 1 == lines.len() && !text.ends_with('\n') => {}
                Err(_) => return Err((i + 1, e.to_string())),
            },
        }
    }
    Ok(content)
}

/// The header line for `store`.
fn header(store: StoreId) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(&Header { store }).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// One record as a line.
pub(crate) fn line(record: &FileOrigin) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(record).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// A file next to `path` named after the whole name of `path` plus `suffix`, so two records files
/// that differ only in their extension (`a.origins`, `a.json`) never share one.
pub(crate) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Write all `records` to a temporary file, sync it, rename it over `path`, sync the folder, and
/// open `path` for appending.
pub(crate) fn rewrite(
    path: &Path,
    records: &BTreeMap<String, FileOrigin>,
    store: Option<StoreId>,
) -> io::Result<File> {
    let temporary = sibling(path, ".tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    if let Some(store) = store {
        file.write_all(&header(store)?)?;
    }
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

/// The lock file next to the records.
pub(crate) fn lock_path(path: &Path) -> PathBuf {
    sibling(path, ".lock")
}

/// Take the exclusive lock of the records at `path` for as long as the returned file lives. The
/// system lets go of it when the program ends, however it ends.
///
/// # Errors
/// `WouldBlock`: another program holds it.
pub(crate) fn lock(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(lock_path(path))?;
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(file),
        Err(Errno::WOULDBLOCK) => Err(io::ErrorKind::WouldBlock.into()),
        Err(e) => Err(e.into()),
    }
}
