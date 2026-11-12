//! The walk of an import: which files are texts, which are attachments, what is not looked at.
//!
//! Pattern: Iterator over a folder tree (a recursive walk that fills the report).

use std::path::{Component, Path};

use crate::hash::digest;
use crate::report::Report;
use crate::source::Failure;

/// Relative paths (as parts) of all Markdown files below `dir`; what is not looked at is listed.
pub(crate) fn collect(root: &Path, dir: &Path, files: &mut Vec<Vec<String>>, report: &mut Report) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            report.failures.push(Failure {
                path: display(root, dir),
                reason: format!("cannot read the folder: {e}"),
            });
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                report.skipped.push(Failure {
                    path: display(root, dir),
                    reason: format!("cannot read an entry of the folder: {e}"),
                });
                continue;
            }
        };
        let path = entry.path();
        let skip = |report: &mut Report, reason: &str| {
            report.skipped.push(Failure {
                path: display(root, &path),
                reason: reason.into(),
            });
        };
        let Ok(kind) = entry.file_type() else {
            skip(report, "cannot tell what kind of entry it is");
            continue;
        };
        let md = path.extension().is_some_and(|e| e == "md");
        let other_case = path
            .extension()
            .is_some_and(|e| e != "md" && e.eq_ignore_ascii_case("md"));
        if kind.is_symlink() {
            if path.is_dir() || md || other_case {
                skip(report, "symbolic link, not followed");
            }
        } else if kind.is_dir() {
            if !entry.file_name().to_string_lossy().starts_with('.') {
                collect(root, &path, files, report);
            }
        } else if kind.is_file() && md {
            let parts = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            files.push(parts);
        } else if kind.is_file() && other_case {
            skip(report, "the extension must be written .md (lower case)");
        } else if kind.is_file() && !is_config(&path) {
            // Not a text: named, with its hash, never dropped silently (the wiki has no place for files yet).
            let said = match std::fs::read(&path) {
                Ok(bytes) => format!(
                    "attachment, {} bytes, sha256 {}: not stored by this version, a link to it shows as missing",
                    bytes.len(),
                    digest(&bytes)
                ),
                Err(e) => {
                    format!("attachment that cannot be read ({e}): not stored by this version")
                }
            };
            report.attachments.push(Failure {
                path: display(root, &path),
                reason: said,
            });
        }
    }
}

/// What only configures the reading (`.pages`, read by `nav.rs`): neither a text nor an attachment.
fn is_config(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == ".pages")
}

/// The path relative to `root` as the wiki writes it (always `/`, on every platform).
pub(crate) fn display(root: &Path, p: &Path) -> String {
    wiki_path(p.strip_prefix(root).unwrap_or(p))
}

/// Wiki paths are not platform paths: the parts of a path are joined with `/`, never with the
/// separator of the system. A backslash inside a part (valid on Linux) stays as it is.
pub(crate) fn wiki_path(p: &Path) -> String {
    p.components()
        .map(|c| match c {
            Component::RootDir => String::new(),
            c => c.as_os_str().to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::wiki_path;

    #[test]
    fn parts_are_joined_with_a_slash() {
        let p = PathBuf::from("a").join("notes.txt");
        assert_eq!(wiki_path(&p), "a/notes.txt");
        assert_eq!(
            wiki_path(&PathBuf::from("a").join("b").join("c.svg")),
            "a/b/c.svg"
        );
    }

    #[test]
    fn a_backslash_in_a_part_is_left_alone_where_it_is_a_name_character() {
        if cfg!(unix) {
            assert_eq!(wiki_path(&PathBuf::from("a\\b")), "a\\b");
            assert_eq!(wiki_path(&PathBuf::from("x").join("a\\b")), "x/a\\b");
        }
    }
}
