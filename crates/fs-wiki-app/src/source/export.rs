//! Writing a page out as a file.
//!
//! Pattern: Guard — the only place that writes a file. The path of a text is checked as a path
//! (no `..`, no leading `/`, no backslash: `TextPath`), the file is put below the target and
//! nowhere else, a symbolic link is never followed, and an existing file is only replaced when
//! the caller says so (the reader asks the person first).

use std::path::{Component, Path};

use fs_components_workbenches::SourceError;
use fs_wiki_core::TextPath;

/// Write `markdown` to `target/path`. Returns a message saying where it went.
///
/// # Errors
/// `Failed` for a path that leaves the target or cannot be written; `Conflict` when the file
/// exists with other content and `overwrite` is not set.
pub fn export_markdown_to(
    path: &str,
    target: &str,
    markdown: &str,
    overwrite: bool,
) -> Result<String, SourceError> {
    let checked = TextPath::parse(path).map_err(|e| SourceError::failed(e.to_string()))?;
    let root = Path::new(target);
    let file = root.join(checked.as_str());
    if file.strip_prefix(root).map_or(true, |p| {
        p.components().any(|c| !matches!(c, Component::Normal(_)))
    }) {
        return Err(SourceError::failed("the path leaves the export folder"));
    }
    if file.is_symlink() {
        return Err(SourceError::failed("the target is a symbolic link"));
    }
    if file.exists() {
        let same = std::fs::read_to_string(&file).is_ok_and(|old| old == markdown);
        if same {
            return Ok(format!("Already exported to {}.", file.display()));
        }
        if !overwrite {
            return Err(SourceError::conflict(format!(
                "{} already exists with other content.",
                file.display()
            )));
        }
    }
    if let Some(dir) = file.parent() {
        reject_linked_folders(root, dir)?;
        std::fs::create_dir_all(dir).map_err(|e| SourceError::failed(e.to_string()))?;
    }
    std::fs::write(&file, markdown).map_err(|e| SourceError::failed(e.to_string()))?;
    Ok(format!("Exported to {}.", file.display()))
}

/// No folder between the target and the file may be a symbolic link (it could lead out).
fn reject_linked_folders(root: &Path, dir: &Path) -> Result<(), SourceError> {
    let mut at = root.to_path_buf();
    for part in dir.strip_prefix(root).unwrap_or(dir).components() {
        at.push(part);
        if at.is_symlink() {
            return Err(SourceError::failed(
                "a folder on the way is a symbolic link",
            ));
        }
    }
    Ok(())
}
