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
    /// The order of the pages (`nav.rs`); empty = by path.
    pub(super) nav: crate::nav::Nav,
    /// Reading only: no editing, no export (the first cut of the reading wiki).
    pub(super) read_only: bool,
    /// Pages with language variants (`pages.rs`); none = every text is a page of its own.
    pub(super) pages: Option<super::pages::PagesView>,
    /// The frontend draws the language switch and the notices itself: the page text has no language bar on top.
    /// Shared with the window, which switches it with the frontend.
    pub(super) bare: Arc<std::sync::atomic::AtomicBool>,
    /// The heads of all texts, read once (`heads.rs`); the sidebar is made from them, not from a list of whole documents.
    pub(super) heads: Option<Vec<crate::heads::Head>>,
    /// Called after every saved edit (the window rebuilds its page tree: the title may have changed).
    pub(super) on_saved: Mutex<Option<super::create::SavedHook>>,
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

    pub(super) fn exists(&self, path: &str) -> bool {
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
        if let Some(view) = &self.pages {
            let heads = match &self.heads {
                Some(h) => h.clone(),
                None => self
                    .runtime
                    .block_on(crate::heads::load_or_list(self.texts.as_ref(), None))
                    .map_err(SourceError::failed)?,
            };
            out.extend(self.page_entries(view, &heads, prefix));
            return Ok(out);
        }
        let mut texts = self
            .runtime
            .block_on(self.texts.list(prefix, Paging::default()))
            .map_err(|e| failed(&e))?;
        // The reader keeps the order it is given: the navigation's, then the rest by path (the list is by path already).
        if !self.nav.is_empty() {
            texts.sort_by_key(|t| self.nav.rank(&t.path).unwrap_or(usize::MAX));
        }
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
        if let Some(view) = &self.pages
            && let Some(doc) = self.read_page(view, path)?
        {
            return Ok(doc);
        }
        let t = self.entry(path)?;
        Ok(to_document(&t.envelope.name, &t.document).with_revision(revision_of(&t)))
    }

    fn resolve(&self, from: &str, href: &str) -> LinkTarget {
        let Some(view) = &self.pages else {
            return resolve_with_folders(from, href, &|p| self.exists(p), &|folder| {
                self.first_in(folder)
            });
        };
        // The language bar: `?lang=de` opens this page in German and makes German the language of the next pages too.
        if let Some(lang) = href.strip_prefix("?lang=") {
            return LinkTarget::Document(view.switch(from, lang));
        }
        // Links are written between the texts (`../x.md` next to `en/a.md`); a page is where its variants are.
        let from_text = view.text_path(from);
        match resolve_with_folders(&from_text, href, &|p| self.exists(p), &|folder| {
            self.first_in(folder)
        }) {
            LinkTarget::Document(text) => LinkTarget::Document(view.page_of(&text)),
            other => other,
        }
    }

    fn find(&self, term: &str) -> Result<Vec<Hit>, SourceError> {
        let found = self
            .runtime
            .block_on(self.texts.find(term, Paging::new(MAX_HITS, 0)))
            .map_err(|e| failed(&e))?;
        let Some(view) = &self.pages else {
            return Ok(found
                .into_iter()
                .map(|t| Hit::new(t.path, t.envelope.name))
                .collect());
        };
        // A page found in both languages is one hit.
        let mut seen = std::collections::HashSet::new();
        Ok(found
            .into_iter()
            .map(|t| Hit::new(view.page_of(&t.path), t.envelope.name))
            .filter(|h| seen.insert(h.path.clone()))
            .collect())
    }

    fn page_info(&self, path: &str) -> fs_components_workbenches::PageInfo {
        self.info_of(path)
    }

    fn start(&self) -> Option<String> {
        if self.exists("index.md") {
            return Some("index.md".to_string());
        }
        // Several sources: the first page of the navigation (`en/index.md`), else the first page there is.
        self.entries("")
            .ok()?
            .into_iter()
            .map(|e| e.path)
            .find(|p| p != PROBLEMS_PATH)
    }

    fn can_edit(&self, path: &str) -> bool {
        !self.read_only && path != PROBLEMS_PATH
    }

    fn source_text(&self, path: &str) -> Result<EditableText, SourceError> {
        let entry = self.entry(&self.text_for(path))?;
        let text = self
            .runtime
            .block_on(self.texts.export_text(entry.id, MARKDOWN))
            .map_err(|e| failed(&e))?;
        Ok(EditableText::new(text, Some(revision_of(&entry))))
    }

    /// The edit goes to the database as the difference to the stored text (`Texts::edit`); layers
    /// stay with the text. No file is written.
    fn save(&self, path: &str, text: &str, revision: Option<&str>) -> Result<(), SourceError> {
        let entry = self.entry(&self.text_for(path))?;
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
            Ok(_) => {
                self.saved();
                Ok(())
            }
            Err(Error::Conflict { .. }) => Err(conflict()),
            Err(e) => Err(failed(&e)),
        }
    }

    fn can_export(&self, path: &str) -> bool {
        !self.read_only && path != PROBLEMS_PATH
    }

    fn export(&self, path: &str, target: &str, overwrite: bool) -> Result<String, SourceError> {
        self.export_page(path, target, overwrite)
    }
}
