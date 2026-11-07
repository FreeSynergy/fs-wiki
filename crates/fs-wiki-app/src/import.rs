//! Importing a folder of Markdown files: the walk.
//!
//! Pattern: Template method (collect files, sync each, report) over the wiki. The folder is only
//! read, never written (ADR 0042: files are exports); the path of a text is the file's path
//! relative to the folder. Every case needs no file to be parsed unless it changed (`sync.rs`).
//! A file that fails is recorded with its reason and the rest goes on; entries that are not
//! looked at (symbolic links, `.MD`, unreadable ones) are listed with the reason.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use fs_store::ObjectId;
use fs_wiki_core::{FileOrigin, Origins, Texts};
use futures::StreamExt;

use crate::report::Report;
use crate::source::Failure;
use crate::sync::sync_file;
use crate::words::{self, say};

/// How long the comparison of the records with the server's texts may take.
pub const CHECK_LIMIT: Duration = Duration::from_secs(60);

/// Bring the wiki in step with the Markdown files under `root` (see `sync.rs` for the rules).
pub async fn import_folder<W: Texts + Origins + ?Sized>(wiki: &W, root: &Path) -> Report {
    import_folder_within(wiki, root, CHECK_LIMIT).await
}

/// [`import_folder`] with another limit for the comparison of the records with the server. A server
/// that does not answer in time ends the import with a failure that says so: nothing is imported.
pub async fn import_folder_within<W: Texts + Origins + ?Sized>(
    wiki: &W,
    root: &Path,
    limit: Duration,
) -> Report {
    let mut report = Report::default();
    let mut files = Vec::new();
    collect(root, root, &mut files, &mut report);
    files.sort();
    let mut known = match wiki.origins().await {
        Ok(all) => all
            .into_iter()
            .map(|o| (o.file.clone(), o))
            .collect::<HashMap<String, FileOrigin>>(),
        Err(e) => {
            report.failures.push(Failure {
                path: String::new(),
                reason: format!("cannot read the record of earlier imports: {e}"),
            });
            return report;
        }
    };
    let present = match present_ids(wiki, &known, limit).await {
        Ok(ids) => ids,
        Err(reason) => {
            report.failures.push(Failure {
                path: String::new(),
                reason,
            });
            return report;
        }
    };
    let mut live = known.values().filter(|o| !o.deleted).peekable();
    if live.peek().is_some() && !live.any(|o| present.contains(&o.text)) {
        // The server holds none of the texts the records name (those deleted here on purpose do not
        // count): they are of another wiki, and say nothing about this one.
        report.records_ignored = known.len();
        known.clear();
    }
    let mut seen = std::collections::HashSet::new();
    for relative in files {
        let path = relative.join("/");
        let file = root.join(relative.iter().collect::<PathBuf>());
        sync_file(wiki, &path, &file, known.get(&path), &present, &mut report).await;
        seen.insert(path);
    }
    for (file, origin) in known {
        if seen.contains(&file) || report.skipped.iter().any(|s| s.path == file) {
            continue;
        }
        if let Ok(Some(text)) = wiki.get(origin.text).await {
            report.gone.push(Failure {
                path: file,
                reason: format!("the file is gone; its text {:?} is kept", text.path),
            });
        }
    }
    report.gone.sort_by(|a, b| a.path.cmp(&b.path));
    report
}

/// How many reads of texts are on their way at once when the records are checked.
const CHECK_IN_FLIGHT: usize = 64;

/// The ids of the records whose texts the server has. The contract has no cheap way to ask this (a
/// listing carries every document and took 5.5 s for 1398 pages), so each record is one `get`, many
/// at once on the one connection. Nothing is asked when no record exists (a first start). The whole
/// check is bounded by `limit`.
async fn present_ids<W: Texts + ?Sized>(
    wiki: &W,
    known: &HashMap<String, FileOrigin>,
    limit: Duration,
) -> Result<HashSet<ObjectId>, String> {
    let ids: Vec<ObjectId> = known.values().map(|o| o.text).collect();
    let reads = futures::stream::iter(ids)
        .map(|id| async move { (id, wiki.get(id).await) })
        .buffer_unordered(CHECK_IN_FLIGHT)
        .collect::<Vec<_>>();
    let Ok(answers) = tokio::time::timeout(limit, reads).await else {
        return Err(say(words::CHECK_TIMEOUT, &[("seconds", &limit.as_secs())]));
    };
    let mut present = HashSet::new();
    for (id, answer) in answers {
        match answer {
            Ok(Some(_)) => {
                present.insert(id);
            }
            Ok(None) => {}
            Err(e) => return Err(say(words::CHECK_FAILED, &[("error", &e)])),
        }
    }
    Ok(present)
}

/// Relative paths (as parts) of all Markdown files below `dir`; what is not looked at is listed.
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
        }
    }
}

fn display(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}
