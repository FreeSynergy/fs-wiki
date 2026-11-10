//! The file system part of the export: no link is ever followed, a file appears whole or not at all.
//!
//! Pattern: Guard — every step goes through an open directory handle (`cap-std`, so `openat` with `O_NOFOLLOW` on Unix and
//! the handle based calls of Windows), so a link swapped in after a check is met by the open itself, not by an earlier look.
//! There is no path check anywhere. A file is written under a temporary name in its folder and renamed over the target, so a
//! crash leaves the old file or the new one, and a hard link to a file elsewhere is replaced, not written through.
//! Strategy by platform: mode and owner of a replaced file are kept where the system has them (Unix); Windows keeps neither.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, DirBuilder, OpenOptions};

/// Why a step failed.
pub enum FsError {
    /// A symbolic link (or something that is not a folder or file) is in the way.
    Linked(String),
    /// A folder on the way does not exist (and was not to be made).
    Missing,
    /// Anything else.
    Other(String),
}

fn other(e: impl std::fmt::Display) -> FsError {
    FsError::Other(e.to_string())
}

/// What a name in `dir` is, without following it: a link or a thing that is no folder/file is the guard's business.
fn linked(dir: &Dir, name: &str, want_dir: bool) -> Option<FsError> {
    let meta = dir.symlink_metadata(name).ok()?;
    let fine = if want_dir {
        meta.is_dir()
    } else {
        meta.is_file()
    };
    if fine {
        None
    } else if meta.is_symlink() {
        Some(FsError::Linked(format!("{name} is a symbolic link")))
    } else if want_dir {
        Some(FsError::Linked(format!(
            "{name} is a symbolic link or not a folder"
        )))
    } else {
        Some(FsError::Linked(format!("{name} is not a regular file")))
    }
}

/// The folder `root` (the target itself may be reached through a link: the person chose it).
pub fn open_root(root: &Path) -> Result<Dir, FsError> {
    Dir::open_ambient_dir(root, ambient_authority()).map_err(other)
}

/// The folder `name` inside `dir`, created if missing; a link in its place is refused.
pub fn open_folder(dir: &Dir, name: &str, create: bool) -> Result<Dir, FsError> {
    match dir.open_dir_nofollow(name) {
        Ok(d) => Ok(d),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if !create {
                return Err(FsError::Missing);
            }
            let mut builder = DirBuilder::new();
            #[cfg(unix)]
            cap_std::fs::DirBuilderExt::mode(&mut builder, 0o755);
            dir.create_dir_with(name, &builder).map_err(other)?;
            dir.open_dir_nofollow(name).map_err(other)
        }
        Err(e) => Err(linked(dir, name, true).unwrap_or_else(|| other(e))),
    }
}

/// A file that is there: its text, and the mode and owner a replacement must keep.
pub struct Existing {
    /// What the file says.
    pub text: String,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    gid: u32,
}

/// The file `name` in `dir`, if it exists; a link or a special file is refused.
pub fn read_existing(dir: &Dir, name: &str) -> Result<Option<Existing>, FsError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = match dir.open_with(name, &options) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(linked(dir, name, false).unwrap_or_else(|| other(e))),
    };
    let meta = file.metadata().map_err(other)?;
    if !meta.is_file() {
        return Err(FsError::Linked(format!("{name} is not a regular file")));
    }
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(other)?;
    Ok(Some(Existing {
        text,
        #[cfg(unix)]
        mode: cap_std::fs::MetadataExt::mode(&meta) & 0o7777,
        #[cfg(unix)]
        uid: cap_std::fs::MetadataExt::uid(&meta),
        #[cfg(unix)]
        gid: cap_std::fs::MetadataExt::gid(&meta),
    }))
}

/// A temporary name that is short whatever the file is called (names are limited to 255 bytes).
fn temp_names() -> impl Iterator<Item = String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let first = COUNTER.fetch_add(1000, Ordering::Relaxed);
    (first..first + 1000).map(|n| format!(".fs-wiki-export-{}-{n}", std::process::id()))
}

/// The mode and owner of the replaced file, onto the temporary one (Unix; the owner is best effort). Owner first (it may
/// clear setuid bits), then the mode.
#[cfg(unix)]
fn keep_rights(file: &cap_std::fs::File, old: &Existing) -> std::io::Result<()> {
    use rustix::fs::{Gid, Uid, fchown};
    let _ = fchown(
        file,
        Some(Uid::from_raw(old.uid)),
        Some(Gid::from_raw(old.gid)),
    );
    file.set_permissions(cap_std::fs::PermissionsExt::from_mode(old.mode))
}

/// Windows keeps no mode and owner of its own to carry over.
#[cfg(not(unix))]
fn keep_rights(_file: &cap_std::fs::File, _old: &Existing) -> std::io::Result<()> {
    Ok(())
}

/// Make what was written durable: the folder itself on Unix (Windows cannot sync a folder).
fn sync_dir(dir: &Dir) -> Result<(), FsError> {
    #[cfg(unix)]
    // `cap-std` holds its folders as path handles that cannot be synced: open the folder again, as a file.
    dir.open(".")
        .and_then(|folder| folder.sync_all())
        .map_err(other)?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Write `text` as `name` in `dir`: a temporary file, synced, then renamed over the target. A file
/// that is replaced (`keep`) keeps its mode, and its owner where we may (Unix); the temporary file is
/// private until then. A new file gets the usual mode (the umask applies).
pub fn write_atomic(
    dir: &Dir,
    name: &str,
    text: &str,
    keep: Option<&Existing>,
) -> Result<(), FsError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    cap_std::fs::OpenOptionsExt::mode(&mut options, if keep.is_some() { 0o600 } else { 0o666 });
    let mut made = None;
    for temp in temp_names() {
        match dir.open_with(&temp, &options) {
            Ok(file) => {
                made = Some((temp, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(other(e)),
        }
    }
    let (temp, mut file) = made.ok_or_else(|| other("no free temporary name"))?;
    if let Some(old) = keep
        && let Err(e) = keep_rights(&file, old)
    {
        let _ = dir.remove_file(&temp);
        return Err(other(e));
    }
    let result = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            drop(file);
            dir.rename(&temp, dir, name)
        })
        .map_err(other);
    if result.is_err() {
        let _ = dir.remove_file(&temp);
    }
    result?;
    sync_dir(dir)
}
