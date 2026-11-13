//! Building a `WikiSource` and its small accessors.
//!
//! Pattern: Builder — `new` or `from_report`, then optionally `with_origins`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fs_components_workbenches::SourceError;
use fs_wiki_core::{Origins, TextEntry, Texts};
use tokio::runtime::Handle;

use super::export::Differs;
use super::problems::{Failure, Notes};
use super::wiki::WikiSource;
use crate::report::Report;

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
            origins: None,
            difference: Mutex::new(None),
            nav: crate::nav::Nav::default(),
            read_only: false,
            pages: None,
            bare: Arc::default(),
            heads: None,
        }
    }

    /// A source that also shows the conflicts, gone files and skipped entries of `report`.
    pub fn from_report(texts: Arc<dyn Texts>, runtime: Handle, report: Report) -> Self {
        let notes = Notes {
            failures: report.failures,
            conflicts: report.conflicts,
            gone: report.gone,
            skipped: report.skipped,
            missing: report.missing,
            attachments: report.attachments,
        };
        Self {
            texts,
            runtime,
            notes,
            origins: None,
            difference: Mutex::new(None),
            nav: crate::nav::Nav::default(),
            read_only: false,
            pages: None,
            bare: Arc::default(),
            heads: None,
        }
    }

    /// Keep the records of `origins` up to date when a page is exported into `docs`, and report an
    /// old file left behind by a move.
    #[must_use]
    pub fn with_origins(mut self, origins: Arc<dyn Origins>, docs: PathBuf) -> Self {
        self.origins = Some((origins, docs));
        self
    }

    /// List the pages in the order of `nav`.
    #[must_use]
    pub fn with_nav(mut self, nav: crate::nav::Nav) -> Self {
        self.nav = nav;
        self
    }

    /// The pages with language variants, and the language to read in first.
    #[must_use]
    pub fn with_pages(mut self, index: crate::variants::PageIndex, lang: &str) -> Self {
        self.pages = Some(super::pages::PagesView::new(index, lang));
        self
    }

    /// The heads of all texts, read before (the sidebar is made from them).
    #[must_use]
    pub fn with_heads(mut self, heads: Vec<crate::heads::Head>) -> Self {
        self.heads = Some(heads);
        self
    }

    /// The frontend draws the language switch and the notices itself (from `page_info`): the text gets no language bar on top.
    #[must_use]
    pub fn bare_pages(self) -> Self {
        self.bare.store(true, std::sync::atomic::Ordering::Relaxed);
        self
    }

    /// The switch of `bare_pages`, for a window that changes its frontend while it runs (`true` = bare).
    pub fn bare_switch(&self) -> Arc<std::sync::atomic::AtomicBool> {
        Arc::clone(&self.bare)
    }

    /// Reading only: the pages cannot be edited or exported from the window.
    #[must_use]
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// The texts of the last export that found a file with other content (both versions).
    pub fn last_difference(&self) -> Option<Differs> {
        self.difference
            .lock()
            .ok()?
            .as_ref()
            .map(|a| a.differs.clone())
    }

    pub(super) fn text_entry(&self, path: &str) -> Result<TextEntry, SourceError> {
        self.entry(path)
    }

    pub(super) fn runtime(&self) -> &Handle {
        &self.runtime
    }
}
