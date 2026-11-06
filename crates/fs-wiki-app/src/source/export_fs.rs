//! The file system part of the export: no link is ever followed, a file appears whole or not at all.
//!
//! Pattern: Guard — every step goes through an open directory (`openat`) with `O_NOFOLLOW`, so a
//! link swapped in after a check is met by the open itself, not by an earlier look. A file is
//! written under a temporary name in its folder and renamed over the target, so a crash leaves
//! the old file or the new one, and a hard link to a file elsewhere is replaced, not written through.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rustix::fs::{
    AtFlags, CWD, FileType, Gid, Mode, OFlags, Uid, fchmod, fchown, fstat, fsync, mkdirat, openat,
    renameat, unlinkat,
};
use rustix::io::Errno;

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

/// The folder `root` (the target itself may be reached through a link: the person chose it).
pub fn open_root(root: &Path) -> Result<OwnedFd, FsError> {
    openat(
        CWD,
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(other)
}

/// The folder `name` inside `dir`, created if missing; a link in its place is refused.
pub fn open_folder(dir: &OwnedFd, name: &str, create: bool) -> Result<OwnedFd, FsError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let open = || openat(dir, name, flags, Mode::empty());
    match open() {
        Err(Errno::NOENT) if !create => Err(FsError::Missing),
        Err(Errno::NOENT) => {
            mkdirat(dir, name, Mode::from_raw_mode(0o755)).map_err(other)?;
            open().map_err(other)
        }
        Err(Errno::LOOP | Errno::NOTDIR) => Err(FsError::Linked(format!(
            "{name} is a symbolic link or not a folder"
        ))),
        r => r.map_err(other),
    }
}

/// A file that is there: its text, and the mode and owner a replacement must keep.
pub struct Existing {
    /// What the file says.
    pub text: String,
    /// Permission bits (including setuid and the like).
    pub mode: u32,
    uid: u32,
    gid: u32,
}

/// The file `name` in `dir`, if it exists; a link or a special file is refused.
pub fn read_existing(dir: &OwnedFd, name: &str) -> Result<Option<Existing>, FsError> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let fd = match openat(dir, name, flags, Mode::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Ok(None),
        Err(Errno::LOOP) => return Err(FsError::Linked(format!("{name} is a symbolic link"))),
        Err(e) => return Err(other(e)),
    };
    let stat = fstat(&fd).map_err(other)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(FsError::Linked(format!("{name} is not a regular file")));
    }
    let mut text = String::new();
    File::from(fd).read_to_string(&mut text).map_err(other)?;
    Ok(Some(Existing {
        text,
        mode: stat.st_mode & 0o7777,
        uid: stat.st_uid,
        gid: stat.st_gid,
    }))
}

/// A temporary name that is short whatever the file is called (names are limited to 255 bytes).
fn temp_names() -> impl Iterator<Item = String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let first = COUNTER.fetch_add(1000, Ordering::Relaxed);
    (first..first + 1000).map(|n| format!(".fs-wiki-export-{}-{n}", std::process::id()))
}

/// Write `text` as `name` in `dir`: a temporary file, synced, then renamed over the target. A file
/// that is replaced (`keep`) keeps its mode, and its owner where we may; the temporary file is
/// private until then. A new file gets the usual mode (the umask applies).
pub fn write_atomic(
    dir: &OwnedFd,
    name: &str,
    text: &str,
    keep: Option<&Existing>,
) -> Result<(), FsError> {
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let create = Mode::from_raw_mode(if keep.is_some() { 0o600 } else { 0o666 });
    let mut made = None;
    for temp in temp_names() {
        match openat(dir, &temp, flags, create) {
            Ok(fd) => {
                made = Some((temp, fd));
                break;
            }
            Err(Errno::EXIST) => {}
            Err(e) => return Err(other(e)),
        }
    }
    let (temp, fd) = made.ok_or_else(|| other("no free temporary name"))?;
    if let Some(old) = keep {
        // Owner first (it may clear setuid bits), then the mode; the owner is best effort.
        let _ = fchown(
            &fd,
            Some(Uid::from_raw(old.uid)),
            Some(Gid::from_raw(old.gid)),
        );
        let restored = fchmod(&fd, Mode::from_raw_mode(old.mode));
        if let Err(e) = restored {
            let _ = unlinkat(dir, &temp, AtFlags::empty());
            return Err(other(e));
        }
    }
    let mut file = File::from(fd);
    let result = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(other)
        .and_then(|()| renameat(dir, &temp, dir, name).map_err(other));
    if result.is_err() {
        let _ = unlinkat(dir, &temp, AtFlags::empty());
    }
    result?;
    fsync(dir).map_err(other)
}
