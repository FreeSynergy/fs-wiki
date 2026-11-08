//! Importing a folder of Markdown files: the walk.
//!
//! Pattern: Template method (collect files, sync each, report) over the wiki. The folder is only
//! read, never written (ADR 0042: files are exports); the path of a text is the file's path
//! relative to the folder. Every case needs no file to be parsed unless it changed (`sync.rs`).
//! A file that fails is recorded with its reason and the rest goes on; entries that are not
//! looked at (symbolic links, `.MD`, unreadable ones) are listed with the reason.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fs_objects::StoreId;
use fs_wiki_core::{FileOrigin, Origins, Texts};

use crate::check::present_ids;
use crate::report::Report;
use crate::source::Failure;
use crate::sync::sync_file;

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
    // Whose records they are is the store's identity (`Texts::identity`): the records of the store
    // we are connected to are used, those of other stores are kept in the file and not used.
    let identity = wiki.identity();
    let mut foreign = match wiki.belong_to(identity).await {
        Ok(foreign) => foreign,
        Err(e) => return failed_start(report, &e),
    };
    if identity.is_none() && foreign.others > 0 {
        // A file made for stores, and a server that tells none: its records get a section of their
        // own, so they are trusted at the next start (the stores' records stay as they are).
        foreign = match wiki.belong_to(Some(unidentified())).await {
            Ok(foreign) => foreign,
            Err(e) => return failed_start(report, &e),
        };
    }
    let mut known = match records_of(wiki).await {
        Ok(known) => known,
        Err(e) => {
            report.failures.push(Failure {
                path: String::new(),
                reason: format!("cannot read the record of earlier imports: {e}"),
            });
            return report;
        }
    };
    let mut checked = None;
    if known.is_empty() {
        report.records_ignored = foreign.others;
        if identity.is_some() && !foreign.legacy.is_empty() {
            // Records from before the store was recorded: they are taken over only if the server has
            // at least one of their texts (one deleted on purpose does not count). Otherwise they
            // may be of another server, and the file is never bound to this one by their account.
            let legacy: HashMap<String, FileOrigin> = foreign
                .legacy
                .into_iter()
                .map(|o| (o.file.clone(), o))
                .collect();
            match present_ids(wiki, &legacy, limit).await {
                Ok((ids, unasked)) => {
                    if legacy.values().any(|o| !o.deleted && ids.contains(&o.text)) {
                        if let Err(e) = wiki.adopt_legacy().await {
                            return failed_start(report, &e);
                        }
                        known = legacy;
                        checked = Some((ids, unasked));
                    } else {
                        report.records_ignored += legacy.len();
                    }
                }
                Err(reason) => {
                    report.failures.push(Failure {
                        path: String::new(),
                        reason,
                    });
                    return report;
                }
            }
        }
    }
    let checked = match checked {
        Some(done) => Ok(done),
        None => present_ids(wiki, &known, limit).await,
    };
    let present = match checked {
        Ok((ids, unasked)) => {
            report.failures.extend(unasked);
            ids
        }
        Err(reason) => {
            report.failures.push(Failure {
                path: String::new(),
                reason,
            });
            return report;
        }
    };
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

/// The section of the records of a provider that tells no identity, in a file that has stores.
fn unidentified() -> StoreId {
    "00000000-0000-4000-8000-000000000000"
        .parse()
        .expect("a UUIDv4 text")
}

/// The records `wiki` answers with, by file.
async fn records_of<W: Origins + ?Sized>(
    wiki: &W,
) -> fs_wiki_core::Result<HashMap<String, FileOrigin>> {
    Ok(wiki
        .origins()
        .await?
        .into_iter()
        .map(|o| (o.file.clone(), o))
        .collect())
}

/// The import cannot start: nothing is imported.
fn failed_start(mut report: Report, why: &dyn std::fmt::Display) -> Report {
    report.failures.push(Failure {
        path: String::new(),
        reason: format!("cannot tell whose the records of earlier imports are: {why}"),
    });
    report
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
