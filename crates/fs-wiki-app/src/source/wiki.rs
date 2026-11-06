//! `WikiSource`.
//!
//! Pattern: Adapter over `Texts` (Repository) — maps entries, reads, finds and link resolution
//! to the workbench's `DataSource`.

use std::path::PathBuf;
use std::sync::Arc;

use fs_components_widgets::Document;
use fs_components_workbenches::{DataSource, Entry, Hit, LinkTarget, ReadError};
use fs_wiki_core::{MARKDOWN, Paging, TextFormat, Texts};
use tokio::runtime::Handle;

use super::problems::{Failure, PROBLEMS_PATH, problems_document};
use super::resolve::resolve;
use crate::map::to_document;

/// Most results of one search.
const MAX_HITS: usize = 200;

/// The wiki behind the reader.
pub struct WikiSource {
    texts: Arc<dyn Texts>,
    runtime: Handle,
    failures: Vec<Failure>,
    export_root: Option<PathBuf>,
}

impl WikiSource {
    /// A source over `texts`; `failures` are the files that did not import (shown as a page).
    pub fn new(texts: Arc<dyn Texts>, runtime: Handle, failures: Vec<Failure>) -> Self {
        Self {
            texts,
            runtime,
            failures,
            export_root: None,
        }
    }

    /// Allow the Export action: it writes the Markdown of a page to `root/<path>`. Nothing else
    /// ever writes into that folder.
    #[must_use]
    pub fn with_export_root(mut self, root: PathBuf) -> Self {
        self.export_root = Some(root);
        self
    }

    fn exists(&self, path: &str) -> bool {
        matches!(
            self.runtime.block_on(self.texts.get_by_path(path)),
            Ok(Some(_))
        )
    }
}

impl DataSource for WikiSource {
    fn entries(&self, prefix: &str) -> Result<Vec<Entry>, ReadError> {
        let mut out = Vec::new();
        if !self.failures.is_empty() && PROBLEMS_PATH.starts_with(prefix) {
            out.push(Entry {
                path: PROBLEMS_PATH.into(),
                title: format!("Import problems ({})", self.failures.len()),
            });
        }
        let texts = self
            .runtime
            .block_on(self.texts.list(prefix, Paging::default()))
            .map_err(|e| ReadError(e.to_string()))?;
        out.extend(texts.into_iter().map(|t| Entry {
            path: t.path,
            title: t.envelope.name,
        }));
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Document, ReadError> {
        if path == PROBLEMS_PATH {
            return Ok(problems_document(&self.failures));
        }
        match self.runtime.block_on(self.texts.get_by_path(path)) {
            Ok(Some(t)) => Ok(to_document(&t.envelope.name, &t.document)),
            Ok(None) => Err(ReadError(format!("there is no page \"{path}\""))),
            Err(e) => Err(ReadError(e.to_string())),
        }
    }

    fn resolve(&self, from: &str, href: &str) -> LinkTarget {
        resolve(from, href, &|p| self.exists(p))
    }

    fn find(&self, term: &str) -> Result<Vec<Hit>, ReadError> {
        let found = self
            .runtime
            .block_on(self.texts.find(term, Paging::new(MAX_HITS, 0)))
            .map_err(|e| ReadError(e.to_string()))?;
        Ok(found
            .into_iter()
            .map(|t| Hit {
                path: t.path,
                title: t.envelope.name,
            })
            .collect())
    }

    fn start(&self) -> Option<String> {
        self.exists("index.md").then(|| "index.md".to_string())
    }

    fn can_edit(&self) -> bool {
        true
    }

    fn source_text(&self, path: &str) -> Result<String, ReadError> {
        let err = |e: &dyn std::fmt::Display| ReadError(e.to_string());
        let entry = self
            .runtime
            .block_on(self.texts.get_by_path(path))
            .map_err(|e| err(&e))?
            .ok_or_else(|| ReadError(format!("there is no page \"{path}\"")))?;
        self.runtime
            .block_on(self.texts.export_text(entry.id, MARKDOWN))
            .map_err(|e| err(&e))
    }

    /// The edit goes to the database as the difference to the stored text (`Texts::edit`); layers
    /// stay with the text. No file is written.
    fn save(&self, path: &str, text: &str) -> Result<(), ReadError> {
        let err = |e: &dyn std::fmt::Display| ReadError(e.to_string());
        let entry = self
            .runtime
            .block_on(self.texts.get_by_path(path))
            .map_err(|e| err(&e))?
            .ok_or_else(|| ReadError(format!("there is no page \"{path}\"")))?;
        let mut document = entry.document.clone();
        if !TextFormat::Markdown.import_external(&mut document, text) {
            return Ok(());
        }
        self.runtime
            .block_on(self.texts.edit(entry.id, document))
            .map(|_| ())
            .map_err(|e| err(&e))
    }

    fn can_export(&self) -> bool {
        self.export_root.is_some()
    }

    fn export(&self, path: &str) -> Result<String, ReadError> {
        let root = self
            .export_root
            .as_ref()
            .ok_or_else(|| ReadError("export is not switched on (--export)".into()))?;
        let markdown = self.source_text(path)?;
        let file = root.join(path);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| ReadError(e.to_string()))?;
        }
        std::fs::write(&file, markdown).map_err(|e| ReadError(e.to_string()))?;
        Ok(format!("Exported to {}.", file.display()))
    }
}
