//! The file format of the records: JSON lines, read, rewritten atomically, locked.
//!
//! Pattern: none — free functions on a file.

use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fs_objects::StoreId;
use fs_wiki_core::FileOrigin;
use serde::{Deserialize, Serialize};

/// The line that starts the records of one store; the records after it are that store's.
#[derive(Serialize, Deserialize)]
struct Header {
    store: StoreId,
}

/// The records of one store (or of none), by file.
pub(crate) type Records = BTreeMap<String, FileOrigin>;

/// What a records file holds: the records from before stores were recorded (`legacy`, at the start
/// of the file, nobody knows whose), and the records of each store.
#[derive(Clone, Default)]
pub(crate) struct Book {
    pub(crate) legacy: Records,
    pub(crate) stores: HashMap<StoreId, Records>,
    /// The store whose header was the last in the file (the end of the file belongs to it).
    pub(crate) tail: Option<StoreId>,
}

/// The records of `text`; the last line may be torn, any other bad line is an error: its number and
/// the parser's reason. A file from before the store id has no header line.
pub(crate) fn parse(text: &str) -> Result<Book, (usize, String)> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut book = Book::default();
    for (i, line) in lines.iter().enumerate() {
        match serde_json::from_str::<FileOrigin>(line) {
            Ok(r) => match book.tail {
                Some(store) => {
                    book.stores
                        .entry(store)
                        .or_default()
                        .insert(r.file.clone(), r);
                }
                None => {
                    book.legacy.insert(r.file.clone(), r);
                }
            },
            Err(e) => match serde_json::from_str::<Header>(line) {
                Ok(header) => {
                    book.stores.entry(header.store).or_default();
                    book.tail = Some(header.store);
                }
                Err(_) if i + 1 == lines.len() && !text.ends_with('\n') => {}
                Err(_) => return Err((i + 1, e.to_string())),
            },
        }
    }
    Ok(book)
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

/// Write the whole `book` to a temporary file, sync it, rename it over `path`, sync the folder, and
/// open `path` for appending. The records of `current` come last, so the end of the file is theirs;
/// the store the end belongs to is returned.
pub(crate) fn rewrite(
    path: &Path,
    book: &Book,
    current: Option<StoreId>,
) -> io::Result<(File, Option<StoreId>)> {
    let temporary = sibling(path, ".tmp");
    let mut file = private_options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)?;
    for record in book.legacy.values() {
        file.write_all(&line(record)?)?;
    }
    let mut others: Vec<_> = book
        .stores
        .keys()
        .filter(|s| Some(**s) != current)
        .collect();
    others.sort_by_key(ToString::to_string);
    let mut tail = None;
    for store in others.into_iter().copied().chain(current) {
        let Some(records) = book.stores.get(&store) else {
            continue;
        };
        file.write_all(&header(store)?)?;
        tail = Some(store);
        for record in records.values() {
            file.write_all(&line(record)?)?;
        }
    }
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        // A folder is synced on Unix; Windows cannot open one as a file, and its rename is durable on its own.
        #[cfg(unix)]
        File::open(dir)?.sync_all()?;
        #[cfg(not(unix))]
        let _ = dir;
    }
    Ok((OpenOptions::new().append(true).open(path)?, tail))
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
    // Read and write: Windows takes a lock only on a handle that can read or write data.
    let file = private_options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path(path))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(io::ErrorKind::WouldBlock.into()),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// Options for a file only the user may read (mode 0600 on Unix; elsewhere the folder's rights).
fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
}
