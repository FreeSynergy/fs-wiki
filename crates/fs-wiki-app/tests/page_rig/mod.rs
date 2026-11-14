//! The rig of the page tests against the real wiki: a window over a wiki with Handbook, its subpage and Notes, and the slot of
//! its modal.

#![allow(dead_code)] // each test file uses a different subset

#[path = "../common/mod.rs"]
mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use fs_components_app::{AppWidget, Status};
use fs_components_workbenches::{Labels, Reader, ReaderWidget, WikiLabels};
use fs_wiki_app::WikiSource;
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{PAGE_RENAME, Setup, window_with_modal};
use fs_wiki_app::source::PageMaker;
use fs_wiki_app::window_modal::ModalSlot;
use fs_wiki_core::{Paging, Texts};
use tokio::runtime::Runtime;

pub struct Rig {
    pub rt: Runtime,
    pub wiki: Arc<dyn Texts>,
    pub app: AppWidget,
    pub reader: ReaderWidget,
    pub modal: Arc<ModalSlot>,
    _fixture: Arc<common::Fixture>,
}

pub fn rig(german: bool) -> Rig {
    let (rt, fixture) = common::wiki();
    let wiki = fixture.wiki().texts();
    for (path, text) in [
        ("handbook.md", "# Handbook\n\nThe handbook.\n"),
        ("handbook/chapter-one.md", "# Chapter one\n\nFirst.\n"),
        ("notes.md", "# Notes\n\nA note.\n"),
    ] {
        rt.block_on(wiki.create(path, text)).unwrap();
    }
    let source = Arc::new(WikiSource::new(
        Arc::clone(&wiki),
        rt.handle().clone(),
        vec![],
    ));
    let maker: Arc<dyn PageMaker> = source.clone();
    let reader = ReaderWidget::new(source, Labels::default(), None);
    reader.with_reader(Reader::refresh_entries);
    let setup = Setup {
        labels: WikiLabels::default(),
        german,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(maker),
    };
    let (app, modal) =
        window_with_modal("Wiki", reader.clone(), Frontend::Confluence, setup).unwrap();
    Rig {
        rt,
        wiki,
        app,
        reader,
        modal,
        _fixture: fixture,
    }
}

impl Rig {
    pub fn paths(&self) -> Vec<String> {
        let all = self
            .rt
            .block_on(self.wiki.list("", Paging::default()))
            .unwrap();
        let mut paths: Vec<String> = all.into_iter().map(|t| t.path).collect();
        paths.sort();
        paths
    }

    pub fn open(&self, path: &str) {
        self.reader.with_reader(|r| r.open(path));
        // The window sees the reader now and then; an entry chosen before that would still be locked.
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(5) {
            let rename_open = self.app.with_shell(|s| {
                s.menus()
                    .iter()
                    .flat_map(|m| m.entries.iter())
                    .any(|e| e.id == PAGE_RENAME && !e.disabled)
            });
            if rename_open {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn status(&self) -> Option<Status> {
        self.app.with_shell(|s| s.status().cloned())
    }

    pub fn text(&self, path: &str) -> String {
        self.rt
            .block_on(async {
                let entry = self.wiki.get_by_path(path).await.ok()??;
                self.wiki
                    .export_text(entry.id, fs_wiki_core::MARKDOWN)
                    .await
                    .ok()
            })
            .unwrap_or_default()
    }
}

impl Rig {
    pub fn make(&self, path: &str, text: &str) {
        self.rt.block_on(self.wiki.create(path, text)).unwrap();
        self.reader.with_reader(Reader::refresh_entries);
    }
}
