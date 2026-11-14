//! The rig of the page tests against the real wiki: a window over a wiki with Handbook, its subpage and Notes, and the slot of
//! its modal.

#![allow(dead_code)] // each test file uses a different subset

#[path = "../common/mod.rs"]
mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use fs_components_app::{AppWidget, Status};
use fs_components_workbenches::{Reader, ReaderWidget};
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
    let reader = ReaderWidget::new(
        source,
        fs_wiki_app::ui_texts::UiTexts::new(german).reader_labels(),
        None,
    );
    reader.with_reader(Reader::refresh_entries);
    let setup = Setup {
        labels: fs_wiki_app::ui_texts::UiTexts::new(german).wiki_labels(),
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

/// The real wiki of the fixture, for tests that build their own source.
pub fn common_wiki() -> (Runtime, Arc<common::Fixture>) {
    common::wiki()
}

impl Rig {
    /// The picture of the window with the styles the program has: Blinc's default look of its own widgets goes ahead of the
    /// theme's rules (`Canvas::with_window_styles`), as the runner of a window does (`blinc_app` `windowed.rs`, `ua_styles::install`).
    /// `AppWidget::picture` does not; its pictures show an unticked checkbox as a bare tick.
    pub fn window_picture(
        &self,
        width: u32,
        height: u32,
        file: &std::path::Path,
    ) -> Result<(), String> {
        let canvas = fs_components_render::Canvas::new(width, height, self.app.styles())
            .with_window_styles();
        fs_components_render::draw_to_png(&canvas, |w, h| self.app.build(w, h), || {}, file)
            .map_err(|e| e.to_string())
    }
}

/// One test at a time may set Blinc up (process-wide state).
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Rig {
    /// Set Blinc up the way a window does (the picture of the window does it), and the notifier of changed signals.
    pub fn warm_up(&self) {
        let warm = std::env::temp_dir().join(format!("fs-wiki-warm-{}.png", std::process::id()));
        let _ = self.app.picture(1000, 700, &warm);
        let _ = std::fs::remove_file(&warm);
        fs_components::blinc::core::reactive::set_stateful_deps_notifier(|ids| {
            fs_components::blinc::layout::check_stateful_deps(ids);
        });
    }
}
