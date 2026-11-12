//! What a sync of the folder did.
//!
//! Pattern: Value objects — plain data the shell shows: counts, and every case that needs a
//! person (failures, conflicts, files that are gone, entries that were skipped) with its reason.

use crate::source::Failure;

/// A file and its text changed since they were last in step. Nothing was overwritten: the
/// database text stays, this is the file's version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The file, relative to the folder.
    pub file: String,
    /// The path of the text now (it may have moved since the import); `None` if it was deleted.
    pub text_path: Option<String>,
    /// What the file says now.
    pub file_text: String,
    /// What is in conflict.
    pub reason: String,
}

/// What an import (sync) of a folder did.
#[derive(Debug, Default)]
pub struct Report {
    /// New files, now texts.
    pub imported: usize,
    /// Files changed outside whose unchanged text took the change as an edit.
    pub updated: usize,
    /// Files that were as last imported or exported (not even parsed).
    pub unchanged: usize,
    /// Files that failed, with the reason.
    pub failures: Vec<Failure>,
    /// Files and texts that both changed: kept as the database has them.
    pub conflicts: Vec<Conflict>,
    /// Files that were imported once and are not there now; their texts are kept.
    pub gone: Vec<Failure>,
    /// Entries that were not looked at, with the reason (symbolic links, `.MD`, unreadable).
    pub skipped: Vec<Failure>,
    /// Files that are not texts (images, style sheets): named with their size and hash, not stored by this version.
    pub attachments: Vec<Failure>,
    /// Files as recorded whose text the server does not have: not imported again, listed.
    pub missing: Vec<Failure>,
    /// Records of imported files that were ignored because the server holds none of their texts:
    /// it is another wiki than the one they were made for (0 = the records fit).
    pub records_ignored: usize,
}

impl Report {
    /// Whether anything needs the person's attention.
    pub fn has_problems(&self) -> bool {
        !(self.failures.is_empty()
            && self.conflicts.is_empty()
            && self.gone.is_empty()
            && self.missing.is_empty()
            && self.skipped.is_empty()
            && self.attachments.is_empty())
    }

    /// Add what `other` found to this report (several sources, one wiki).
    pub fn absorb(&mut self, other: Report) {
        self.imported += other.imported;
        self.updated += other.updated;
        self.unchanged += other.unchanged;
        self.records_ignored += other.records_ignored;
        self.failures.extend(other.failures);
        self.conflicts.extend(other.conflicts);
        self.gone.extend(other.gone);
        self.skipped.extend(other.skipped);
        self.attachments.extend(other.attachments);
        self.missing.extend(other.missing);
    }
}
