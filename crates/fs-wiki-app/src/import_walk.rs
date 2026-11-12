//! The walk of an import: which files are texts, which are attachments, what is not looked at.
//!
//! Pattern: Iterator over a folder tree (a recursive walk that fills the report).

use std::path::Path;

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

pub(crate) fn display(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}
