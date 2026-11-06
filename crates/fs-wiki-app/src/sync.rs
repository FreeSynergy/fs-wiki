//! One file against the database: what to do, from what each looked like when last in step.
//!
//! Pattern: State (decision table) over a `FileOrigin` memento. The database is the truth
//! (ADR 0042), so nothing is ever overwritten silently:
//!
//! | file | text | does |
//! |---|---|---|
//! | as recorded | any (changed, moved, deleted) | nothing, and the file is not parsed |
//! | changed | as recorded | the change is a normal edit (ADR 0042 §5), the record is renewed |
//! | changed | changed or deleted | the text stays; the file's version is a conflict |
//! | new, no text there | | import, record |
//! | new, a text is there already | equal | record only |
//! | new, a text is there already | different | conflict |
//!
//! A file that is gone is reported by the caller; its text is kept.

use std::path::Path;

use fs_wiki_core::{FileOrigin, MARKDOWN, Origins, TextEntry, TextFormat, Texts};

use crate::hash::digest;
use crate::report::{Conflict, Report};
use crate::source::Failure;

/// The hash of the Markdown of `entry`.
pub fn text_hash(entry: &TextEntry) -> String {
    digest(TextFormat::Markdown.export(&entry.document).as_bytes())
}

/// Sync the file `file` (the text path is `path`) against what is `known` about it.
pub async fn sync_file<W: Texts + Origins + ?Sized>(
    wiki: &W,
    path: &str,
    file: &Path,
    known: Option<&FileOrigin>,
    report: &mut Report,
) {
    let fail = |report: &mut Report, reason: String| {
        report.failures.push(Failure {
            path: path.into(),
            reason,
        });
    };
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => return fail(report, format!("cannot read the file: {e}")),
    };
    let hash = digest(&bytes);
    if known.is_some_and(|o| o.hash == hash) {
        report.unchanged += 1;
        return;
    }
    let Ok(content) = String::from_utf8(bytes) else {
        return fail(report, "cannot read the file: not valid UTF-8".into());
    };
    let outcome = match known {
        Some(o) => changed_file(wiki, o, &hash, &content).await,
        None => new_file(wiki, path, &hash, &content).await,
    };
    match outcome {
        Ok(Outcome::Imported) => report.imported += 1,
        Ok(Outcome::Updated) => report.updated += 1,
        Ok(Outcome::Recorded) => report.unchanged += 1,
        Ok(Outcome::Conflict(text_path, reason)) => report.conflicts.push(Conflict {
            file: path.into(),
            text_path,
            file_text: content,
            reason,
        }),
        Err(e) => fail(report, e.to_string()),
    }
}

enum Outcome {
    Imported,
    Updated,
    Recorded,
    Conflict(Option<String>, String),
}

async fn record<W: Texts + Origins + ?Sized>(
    wiki: &W,
    file: &str,
    hash: &str,
    entry: &TextEntry,
) -> fs_wiki_core::Result<()> {
    let origin = FileOrigin {
        file: file.into(),
        hash: hash.into(),
        text: entry.id,
        text_hash: text_hash(entry),
    };
    wiki.set_origin(&origin).await
}

async fn changed_file<W: Texts + Origins + ?Sized>(
    wiki: &W,
    origin: &FileOrigin,
    hash: &str,
    content: &str,
) -> fs_wiki_core::Result<Outcome> {
    let Some(text) = wiki.get(origin.text).await? else {
        return Ok(Outcome::Conflict(
            None,
            "the file changed, but its text was deleted since; the text stays deleted".into(),
        ));
    };
    if text_hash(&text) == origin.text_hash {
        if content.trim_start_matches('\u{feff}').trim().is_empty() && !text_is_empty(&text) {
            // A file cut to nothing outside is more likely a mishap than an edit.
            let reason = "the file is empty now but its text is not; the text is kept";
            return Ok(Outcome::Conflict(Some(text.path), reason.into()));
        }
        let updated = wiki.import_text(&text.path, MARKDOWN, content).await?;
        record(wiki, &origin.file, hash, &updated).await?;
        return Ok(Outcome::Updated);
    }
    if text_hash(&text) == digest(content.as_bytes()) {
        record(wiki, &origin.file, hash, &text).await?;
        return Ok(Outcome::Recorded);
    }
    let reason = "the file and the text were both changed since the last import; the text is kept";
    Ok(Outcome::Conflict(Some(text.path), reason.into()))
}

async fn new_file<W: Texts + Origins + ?Sized>(
    wiki: &W,
    path: &str,
    hash: &str,
    content: &str,
) -> fs_wiki_core::Result<Outcome> {
    let Some(text) = wiki.get_by_path(path).await? else {
        let created = wiki.import_text(path, MARKDOWN, content).await?;
        record(wiki, path, hash, &created).await?;
        return Ok(Outcome::Imported);
    };
    if text_hash(&text) == digest(content.as_bytes()) {
        record(wiki, path, hash, &text).await?;
        return Ok(Outcome::Recorded);
    }
    let reason = "a text is already there that differs from the file, and nothing records that \
                  they were ever in step; the text is kept";
    Ok(Outcome::Conflict(Some(text.path), reason.into()))
}

fn text_is_empty(entry: &TextEntry) -> bool {
    TextFormat::Markdown
        .export(&entry.document)
        .trim()
        .is_empty()
}
