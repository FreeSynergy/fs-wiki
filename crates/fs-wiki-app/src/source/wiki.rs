//! `WikiSource`.
//!
//! Pattern: Adapter over `Texts` (Repository) — maps entries, reads, finds, link resolution,
//! editing and export to the workbench's `DocumentSource`.
//!
//! Revision: the text's write counter (`TextEntry::revision`, never goes back whatever the clock
//! does). `save` edits with `Texts::edit_if`, a compare-and-set under the core's write lock, so a
//! second writer between the read and the write gets a conflict instead of being overwritten.
//! The lock is inside one process: two programs writing the same store file are not covered.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, EditableText, Entry, Hit, LinkTarget, SourceError,
};
use fs_wiki_core::{Error, MARKDOWN, Origins, Paging, TextEntry, TextFormat, Texts};
use tokio::runtime::Handle;

use super::export::Differs;
use super::problems::{Notes, PROBLEMS_PATH, problems_document};
use super::resolve::resolve_with_folders;
use crate::map::to_document;

/// Most results of one search.
const MAX_HITS: usize = 200;

/// The question the last export asked: which page, into which folder, about which content. An
/// answer counts only for this very question.
pub(super) struct Asked {
    pub path: String,
    pub target: String,
    pub differs: Differs,
}

/// The wiki behind the reader.
pub struct WikiSource {
    pub(super) texts: Arc<dyn Texts>,
    pub(super) runtime: Handle,
    pub(super) notes: Notes,
    /// The records of imports and the docs folder they belong to (see `with_origins`).
    pub(super) origins: Option<(Arc<dyn Origins>, PathBuf)>,
    /// The difference that made the last export a conflict, for a screen that shows it.
    pub(super) difference: Mutex<Option<Asked>>,
}

fn failed(e: &dyn std::fmt::Display) -> SourceError {
    SourceError::failed(e.to_string())
}

fn revision_of(entry: &TextEntry) -> String {
    entry.revision.to_string()
}

impl WikiSource {
    /// The first page in `folder` (`""` = everything), in path order.
    fn first_in(&self, folder: &str) -> Option<String> {
        let prefix = if folder.is_empty() {
            String::new()
        } else {
            format!("{folder}/")
        };
        let first = self
            .runtime
            .block_on(self.texts.list(&prefix, Paging::new(1, 0)));
        first.ok()?.into_iter().next().map(|t| t.path)
    }

    fn exists(&self, path: &str) -> bool {
        matches!(
            self.runtime.block_on(self.texts.get_by_path(path)),
            Ok(Some(_))
        )
    }

    pub(super) fn entry(&self, path: &str) -> Result<TextEntry, SourceError> {
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
        resolve_with_folders(from, href, &|p| self.exists(p), &|folder| {
            self.first_in(folder)
        })
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
        let conflict = || {
            SourceError::conflict(
                "Someone else changed this document since you started. Your text is kept; \
                 copy it, cancel, and edit the new version.",
            )
        };
        if revision.is_some_and(|r| r != revision_of(&entry)) {
            return Err(conflict());
        }
        let mut document = entry.document.clone();
        if !TextFormat::Markdown.import_external(&mut document, text) {
            return Ok(());
        }
        match self
            .runtime
            .block_on(self.texts.edit_if(entry.id, entry.revision, document))
        {
            Ok(_) => Ok(()),
            Err(Error::Conflict { .. }) => Err(conflict()),
            Err(e) => Err(failed(&e)),
        }
    }

    fn can_export(&self, path: &str) -> bool {
        path != PROBLEMS_PATH
    }

    fn export(&self, path: &str, target: &str, overwrite: bool) -> Result<String, SourceError> {
        self.export_page(path, target, overwrite)
    }
}
