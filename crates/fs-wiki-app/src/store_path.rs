//! Where the records of a docs folder are kept by default.
//!
//! Pattern: none — one pure function. One records file per docs folder, so two folders never mix
//! their records of what was imported. (The texts are the server's; one server is one wiki.)

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::hash::digest;

/// The records file for `docs` below the data directory `data` (`$XDG_DATA_HOME`, else
/// `$HOME/.local/share`): `<data>/freesynergy/fs-wiki/<folder name>-<hash of the full path>.origins`.
pub fn origins_file(data: &Path, docs: &Path) -> PathBuf {
    folder_file(data, docs, "origins")
}

/// The store file earlier versions of the app kept the wiki of `docs` in (before the wiki moved
/// into `fs-wiki-server`): `<data>/freesynergy/fs-wiki/<folder name>-<hash>.db`. It is not read any
/// more; `Some` when it exists, so the app can say so.
pub fn old_store_file(data: &Path, docs: &Path) -> Option<PathBuf> {
    Some(folder_file(data, docs, "db")).filter(|f| f.is_file())
}

fn folder_file(data: &Path, docs: &Path, extension: &str) -> PathBuf {
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
        .join(format!("{name}-{}.{extension}", &hash[..16]))
}

/// The user's data directory, if the environment names one (Windows: `%LOCALAPPDATA%`).
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?)).filter(|p| p.is_absolute())
    }
    #[cfg(not(windows))]
    {
        data_dir_from(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
    }
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
