//! Importing a folder of Markdown files.
//!
//! Pattern: Template method (collect files, import each, report) over the `Texts` trait. The
//! folder is only read, never written (ADR 0042: files are exports); the path of a text is the
//! file's path relative to the folder. A file that fails is recorded with its reason and the rest
//! goes on.

use std::path::{Path, PathBuf};

use fs_wiki_core::{MARKDOWN, Texts};

use crate::source::Failure;

/// What an import did.
#[derive(Debug, Default)]
pub struct Report {
    /// Files imported.
    pub imported: usize,
    /// Files that failed, with the reason.
    pub failures: Vec<Failure>,
}

/// Import every `*.md` file under `root` into `texts`, path = relative path with `/`.
pub async fn import_folder(texts: &dyn Texts, root: &Path) -> Report {
    let mut report = Report::default();
    let mut files = Vec::new();
    collect(root, root, &mut files, &mut report);
    files.sort();
    for relative in files {
        let path = relative.join("/");
        let file = root.join(relative.iter().collect::<PathBuf>());
        match std::fs::read_to_string(&file) {
            Err(e) => report.failures.push(Failure {
                path,
                reason: format!("cannot read the file: {e}"),
            }),
            Ok(text) => match texts.import_text(&path, MARKDOWN, &text).await {
                Ok(_) => report.imported += 1,
                Err(e) => report.failures.push(Failure {
                    path,
                    reason: e.to_string(),
                }),
            },
        }
    }
    report
}

/// Relative paths (as parts) of all Markdown files below `dir`; unreadable folders are failures.
fn collect(root: &Path, dir: &Path, files: &mut Vec<Vec<String>>, report: &mut Report) {
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
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !entry.file_name().to_string_lossy().starts_with('.') {
                collect(root, &path, files, report);
            }
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "md") {
            let parts = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            files.push(parts);
        }
    }
}

fn display(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}
