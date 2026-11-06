//! `WikiSource`.
//!
//! Pattern: Adapter over `Texts` (Repository) — maps entries, reads, finds, link resolution,
//! editing and export to the workbench's `DocumentSource`.
//!
//! Revision: the store's `updated` time of the text. `Texts::edit` has no revision, so `save`
//! compares the revision in the adapter and then edits; this is **not atomic** (another writer
//! can slip in between the compare and the edit; a store-level compare-and-set is a wish for
//! `fs-wiki-core`).

use std::sync::Arc;

use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, EditableText, Entry, Hit, LinkTarget, SourceError,
};
use fs_wiki_core::{MARKDOWN, Paging, TextEntry, TextFormat, Texts};
use tokio::runtime::Handle;

use super::export::export_markdown_to;
use super::problems::{Failure, Notes, PROBLEMS_PATH, problems_document};
use super::resolve::resolve;
use crate::map::to_document;
use crate::report::Report;

/// Most results of one search.
const MAX_HITS: usize = 200;

/// The wiki behind the reader.
pub struct WikiSource {
    texts: Arc<dyn Texts>,
    runtime: Handle,
    notes: Notes,
}

fn failed(e: &dyn std::fmt::Display) -> SourceError {
    SourceError::failed(e.to_string())
}

fn revision_of(entry: &TextEntry) -> String {
    format!("{:?}", entry.envelope.updated)
}

impl WikiSource {
    /// A source over `texts`; `failures` are the files that did not import (shown as a page).
    pub fn new(texts: Arc<dyn Texts>, runtime: Handle, failures: Vec<Failure>) -> Self {
        let notes = Notes {
            failures,
            ..Notes::default()
        };
        Self {
            texts,
            runtime,
            notes,
        }
    }

    /// A source that also shows the conflicts, gone files and skipped entries of `report`.
    pub fn from_report(texts: Arc<dyn Texts>, runtime: Handle, report: Report) -> Self {
        let notes = Notes {
            failures: report.failures,
            conflicts: report.conflicts,
            gone: report.gone,
            skipped: report.skipped,
        };
        Self {
            texts,
            runtime,
            notes,
        }
    }

    fn exists(&self, path: &str) -> bool {
        matches!(
            self.runtime.block_on(self.texts.get_by_path(path)),
            Ok(Some(_))
        )
    }

    fn entry(&self, path: &str) -> Result<TextEntry, SourceError> {
        self.runtime
            .block_on(self.texts.get_by_path(path))
            .map_err(|e| failed(&e))?
            .ok_or_else(|| SourceError::not_found(format!("there is no document \"{path}\"")))
    }
}

impl DocumentSource for WikiSource {
    fn entries(&self, prefix: &str) -> Result<Vec<Entry>, SourceError> {
        let mut out = Vec::new();
        if self.notes.count() > 0 && PROBLEMS_PATH.starts_with(prefix) {
            out.push(Entry::new(
                PROBLEMS_PATH,
                format!("Import problems ({})", self.notes.count()),
            ));
        }
        let texts = self
            .runtime
            .block_on(self.texts.list(prefix, Paging::default()))
            .map_err(|e| failed(&e))?;
        out.extend(
            texts
                .into_iter()
                .map(|t| Entry::new(t.path, t.envelope.name)),
        );
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Document, SourceError> {
        if path == PROBLEMS_PATH {
            return Ok(problems_document(&self.notes));
        }
        let t = self.entry(path)?;
        Ok(to_document(&t.envelope.name, &t.document).with_revision(revision_of(&t)))
    }

    fn resolve(&self, from: &str, href: &str) -> LinkTarget {
        resolve(from, href, &|p| self.exists(p))
    }

    fn find(&self, term: &str) -> Result<Vec<Hit>, SourceError> {
        let found = self
            .runtime
            .block_on(self.texts.find(term, Paging::new(MAX_HITS, 0)))
            .map_err(|e| failed(&e))?;
        Ok(found
            .into_iter()
            .map(|t| Hit::new(t.path, t.envelope.name))
            .collect())
    }

    fn start(&self) -> Option<String> {
        self.exists("index.md").then(|| "index.md".to_string())
    }

    fn can_edit(&self, path: &str) -> bool {
        path != PROBLEMS_PATH
    }

    fn source_text(&self, path: &str) -> Result<EditableText, SourceError> {
        let entry = self.entry(path)?;
        let text = self
            .runtime
            .block_on(self.texts.export_text(entry.id, MARKDOWN))
            .map_err(|e| failed(&e))?;
        Ok(EditableText::new(text, Some(revision_of(&entry))))
    }

    /// The edit goes to the database as the difference to the stored text (`Texts::edit`); layers
    /// stay with the text. No file is written.
    fn save(&self, path: &str, text: &str, revision: Option<&str>) -> Result<(), SourceError> {
        let entry = self.entry(path)?;
        if revision.is_some_and(|r| r != revision_of(&entry)) {
            return Err(SourceError::conflict(
                "Someone else changed this document since you started. Your text is kept; \
                 copy it, cancel, and edit the new version.",
            ));
        }
        let mut document = entry.document.clone();
        if !TextFormat::Markdown.import_external(&mut document, text) {
            return Ok(());
        }
        self.runtime
            .block_on(self.texts.edit(entry.id, document))
            .map(|_| ())
            .map_err(|e| failed(&e))
    }

    fn can_export(&self, path: &str) -> bool {
        path != PROBLEMS_PATH
    }

    fn export(&self, path: &str, target: &str, overwrite: bool) -> Result<String, SourceError> {
        let markdown = self.source_text(path)?.text;
        export_markdown_to(path, target, &markdown, overwrite)
    }
}
