//! Where the wiki of a docs folder is kept by default.
//!
//! Pattern: none — one pure function. One store per docs folder, so two folders never mix their
//! texts or their records of what was imported.

use std::path::{Path, PathBuf};

use crate::hash::digest;

/// The store file for `docs` below the data directory `data` (`$XDG_DATA_HOME`, else
/// `$HOME/.local/share`): `<data>/freesynergy/fs-wiki/<folder name>-<hash of the full path>.db`.
pub fn store_file(data: &Path, docs: &Path) -> PathBuf {
    let full = docs.canonicalize().unwrap_or_else(|_| docs.to_path_buf());
    let name: String = full
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let hash = digest(full.to_string_lossy().as_bytes());
    data.join("freesynergy")
        .join("fs-wiki")
        .join(format!("{name}-{}.db", &hash[..16]))
}

/// The user's data directory, if the environment names one.
pub fn data_dir() -> Option<PathBuf> {
    let var = |k| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local").join("share")))
}
