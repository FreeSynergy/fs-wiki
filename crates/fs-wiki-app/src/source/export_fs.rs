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

use rustix::fs::{
    AtFlags, CWD, FileType, Mode, OFlags, fstat, fsync, mkdirat, openat, renameat, unlinkat,
};
use rustix::io::Errno;

/// Why a step failed.
pub enum FsError {
    /// A symbolic link (or something that is not a folder or file) is in the way.
    Linked(String),
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
        Err(Errno::NOENT) if create => {
            mkdirat(dir, name, Mode::from_raw_mode(0o755)).map_err(other)?;
            open().map_err(other)
        }
        Err(Errno::LOOP | Errno::NOTDIR) => Err(FsError::Linked(format!(
            "{name} is a symbolic link or not a folder"
        ))),
        r => r.map_err(other),
    }
}

/// The text of the file `name` in `dir`, if it exists; a link or a special file is refused.
pub fn read_existing(dir: &OwnedFd, name: &str) -> Result<Option<String>, FsError> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let fd = match openat(dir, name, flags, Mode::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Ok(None),
        Err(Errno::LOOP) => return Err(FsError::Linked(format!("{name} is a symbolic link"))),
        Err(e) => return Err(other(e)),
    };
    if FileType::from_raw_mode(fstat(&fd).map_err(other)?.st_mode) != FileType::RegularFile {
        return Err(FsError::Linked(format!("{name} is not a regular file")));
    }
    let mut text = String::new();
    File::from(fd).read_to_string(&mut text).map_err(other)?;
    Ok(Some(text))
}

/// Write `text` as `name` in `dir`: a temporary file, synced, then renamed over the target.
pub fn write_atomic(dir: &OwnedFd, name: &str, text: &str) -> Result<(), FsError> {
    let temp = format!(".{name}.tmp-{}", std::process::id());
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let fd = openat(dir, &temp, flags, Mode::from_raw_mode(0o644)).map_err(other)?;
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
