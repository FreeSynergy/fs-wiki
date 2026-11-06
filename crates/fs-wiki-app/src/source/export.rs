//! Writing a page out as a file.
//!
//! Pattern: Guard — the only place that writes a file. The path of a text is checked as a path
//! (no `..`, no leading `/`, no backslash: `TextPath`); the file is put below the target and
//! nowhere else, with no link followed at any step (`export_fs.rs`); a file appears whole or not
//! at all; an existing file with other content is a conflict that carries both texts (so a screen
//! can show the difference), and is replaced only when the caller says so.

use std::path::{Path, PathBuf};

use fs_components_workbenches::SourceError;
use fs_wiki_core::TextPath;

use super::export_fs::{FsError, open_folder, open_root, read_existing, write_atomic};

/// A file that exists with other content than the export would write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Differs {
    /// The file.
    pub file: PathBuf,
    /// What is in the file now.
    pub on_disk: String,
    /// What the export would write.
    pub wanted: String,
}

/// Why an export did not write.
#[derive(Debug)]
pub enum ExportError {
    /// The file exists with other content and replacing was not allowed.
    Differs(Differs),
    /// Anything else, with the reason for the person.
    Failed(String),
}

impl From<FsError> for ExportError {
    fn from(e: FsError) -> Self {
        match e {
            FsError::Linked(m) => Self::Failed(format!(
                "a symbolic link or special file is in the way: {m}"
            )),
            FsError::Other(m) => Self::Failed(m),
        }
    }
}

/// Write `markdown` to `target/path`. Returns a message saying where it went.
///
/// # Errors
/// [`ExportError::Failed`] for a path that leaves the target, a link in the way or a write that
/// fails; [`ExportError::Differs`] when the file exists with other content and `overwrite` is not set.
pub fn export_markdown_checked(
    path: &str,
    target: &str,
    markdown: &str,
    overwrite: bool,
) -> Result<String, ExportError> {
    let checked = TextPath::parse(path).map_err(|e| ExportError::Failed(e.to_string()))?;
    let file = Path::new(target).join(checked.as_str());
    let mut parts: Vec<&str> = checked.as_str().split('/').collect();
    let name = parts.pop().unwrap_or_default();
    std::fs::create_dir_all(target).map_err(|e| ExportError::Failed(e.to_string()))?;
    let mut dir = open_root(Path::new(target))?;
    for part in parts {
        dir = open_folder(&dir, part, true)?;
    }
    match read_existing(&dir, name)? {
        Some(old) if old == markdown => {
            return Ok(format!("Already exported to {}.", file.display()));
        }
        Some(old) if !overwrite => {
            return Err(ExportError::Differs(Differs {
                file,
                on_disk: old,
                wanted: markdown.to_string(),
            }));
        }
        _ => {}
    }
    write_atomic(&dir, name, markdown)?;
    Ok(format!("Exported to {}.", file.display()))
}

/// The same for the reader: a conflict becomes a `Conflict` error with a short account of the
/// difference (the texts themselves are in [`export_markdown_checked`]'s error).
///
/// # Errors
/// `Failed` or `Conflict` as above.
pub fn export_markdown_to(
    path: &str,
    target: &str,
    markdown: &str,
    overwrite: bool,
) -> Result<String, SourceError> {
    export_markdown_checked(path, target, markdown, overwrite).map_err(|e| match e {
        ExportError::Failed(m) => SourceError::failed(m),
        ExportError::Differs(d) => SourceError::conflict(format!(
            "{} already exists with other content ({}).",
            d.file.display(),
            summary(&d.on_disk, &d.wanted)
        )),
    })
}

/// "3 of 40 lines differ, the first is line 7".
fn summary(on_disk: &str, wanted: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (on_disk.lines().collect(), wanted.lines().collect());
    let total = a.len().max(b.len());
    let differing: Vec<usize> = (0..total).filter(|&i| a.get(i) != b.get(i)).collect();
    match differing.first() {
        Some(first) => format!(
            "{} of {total} lines differ, the first is line {}",
            differing.len(),
            first + 1
        ),
        None => "only the line endings differ".into(),
    }
}
