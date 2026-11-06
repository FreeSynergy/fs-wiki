//! Where the wiki of a docs folder is kept by default.
//!
//! Pattern: none — one pure function. One store per docs folder, so two folders never mix their
//! texts or their records of what was imported.

use std::ffi::OsString;
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
    data_dir_from(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
}

/// `xdg` (`$XDG_DATA_HOME`) or else `home/.local/share`. As the XDG specification says, a value
/// that is empty or not an absolute path is treated as unset (a relative one would put the data
/// wherever the program happens to run).
pub fn data_dir_from(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let absolute = |v: OsString| Some(PathBuf::from(v)).filter(|p| p.is_absolute());
    xdg.and_then(absolute).or_else(|| {
        home.and_then(absolute)
            .map(|h| h.join(".local").join("share"))
    })
}
