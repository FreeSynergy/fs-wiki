//! Exporting one page of the wiki, and keeping the records of the docs folder right.
//!
//! Pattern: Template method — export through the guard (`export.rs`), then, if the target is the
//! docs folder, record what the file looks like now (so the next start sees it as in step), and
//! name the file an earlier import of the same text left under its old path after a move. Nothing
//! is deleted.

use std::fmt::Write;
use std::path::Path;

use fs_components_workbenches::SourceError;
use fs_wiki_core::{FileOrigin, MARKDOWN};

use super::export::{ExportError, export_markdown_checked};
use super::wiki::WikiSource;
use crate::hash::digest;

fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

impl WikiSource {
    pub(super) fn export_page(
        &self,
        path: &str,
        target: &str,
        overwrite: bool,
    ) -> Result<String, SourceError> {
        let entry = self.text_entry(path)?;
        let markdown = self
            .runtime()
            .block_on(fs_wiki_core::Texts::export_text(
                self.texts(),
                entry.id,
                MARKDOWN,
            ))
            .map_err(|e| SourceError::failed(e.to_string()))?;
        if let Ok(mut slot) = self.difference.lock() {
            *slot = None;
        }
        let mut message = match export_markdown_checked(path, target, &markdown, overwrite) {
            Ok(m) => m,
            Err(ExportError::Failed(m)) => return Err(SourceError::failed(m)),
            Err(ExportError::Differs(d)) => {
                let message = super::export::export_markdown_to(path, target, &markdown, false)
                    .err()
                    .map_or_else(String::new, |e| e.message);
                if let Ok(mut slot) = self.difference.lock() {
                    *slot = Some(d);
                }
                return Err(SourceError::conflict(message));
            }
        };
        let Some((origins, docs)) = &self.origins else {
            return Ok(message);
        };
        if !same_folder(docs, Path::new(target)) {
            return Ok(message);
        }
        let all = self
            .runtime()
            .block_on(origins.origins())
            .unwrap_or_default();
        let hash = digest(markdown.as_bytes());
        let record = FileOrigin {
            file: path.to_string(),
            hash: hash.clone(),
            text: entry.id,
            text_hash: hash,
        };
        let _ = self.runtime().block_on(origins.set_origin(&record));
        for old in all.iter().filter(|o| o.text == entry.id && o.file != path) {
            if docs.join(&old.file).exists() {
                let _ = write!(
                    message,
                    " The old file {} from before the move was left in place; remove it yourself if it is not needed.",
                    old.file
                );
            }
        }
        Ok(message)
    }
}
