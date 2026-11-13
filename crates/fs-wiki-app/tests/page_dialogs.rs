//! The menu Page against the real wiki (server, store, history): make, subpage, rename, delete, discard, each through the
//! dialog of the window and answered as a person would (name or button), then read back from the wiki.
//!
//! Pattern: Integration test over the public API, no window is drawn (no graphics needed); the dialogs are answered through
//! the slot of the modal, which calls what the buttons and keys call.

mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use fs_components_app::{AppWidget, Status};
use fs_components_workbenches::{Labels, Reader, ReaderWidget, WikiLabels};
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{
    PAGE_DELETE, PAGE_DISCARD, PAGE_NEW, PAGE_RENAME, PAGE_SUB, Setup, window_with_modal,
};
use fs_wiki_app::source::PageMaker;
use fs_wiki_app::window_modal::ModalSlot;
use fs_wiki_app::WikiSource;
use fs_wiki_core::{Paging, Texts};
use tokio::runtime::Runtime;

struct Rig {
    rt: Runtime,
    wiki: Arc<dyn Texts>,
    app: AppWidget,
    reader: ReaderWidget,
    modal: Arc<ModalSlot>,
    _fixture: Arc<common::Fixture>,
}

fn rig(german: bool) -> Rig {
    let (rt, fixture) = common::wiki();
    let wiki = fixture.wiki().texts();
    for (path, text) in [
        ("handbook.md", "# Handbook\n\nThe handbook.\n"),
        ("handbook/chapter-one.md", "# Chapter one\n\nFirst.\n"),
        ("notes.md", "# Notes\n\nA note.\n"),
    ] {
        rt.block_on(wiki.create(path, text)).unwrap();
    }
    let source = Arc::new(WikiSource::new(Arc::clone(&wiki), rt.handle().clone(), vec![]));
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
    let (app, modal) = window_with_modal("Wiki", reader.clone(), Frontend::Confluence, setup).unwrap();
    Rig { rt, wiki, app, reader, modal, _fixture: fixture }
}

impl Rig {
    fn paths(&self) -> Vec<String> {
        let all = self.rt.block_on(self.wiki.list("", Paging::default())).unwrap();
        let mut paths: Vec<String> = all.into_iter().map(|t| t.path).collect();
        paths.sort();
        paths
    }

    fn open(&self, path: &str) {
        self.reader.with_reader(|r| r.open(path));
        // The window sees the reader now and then; an entry chosen before that would still be locked.
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(5) {
            let rename_open = self.app.with_shell(|s| {
                s.menus().iter().flat_map(|m| m.entries.iter()).any(|e| e.id == PAGE_RENAME && !e.disabled)
            });
            if rename_open {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn status(&self) -> Option<Status> {
        self.app.with_shell(|s| s.status().cloned())
    }

    fn text(&self, path: &str) -> String {
        self.rt.block_on(async {
            let entry = self.wiki.get_by_path(path).await.ok()??;
            self.wiki.export_text(entry.id, fs_wiki_core::MARKDOWN).await.ok()
        }).unwrap_or_default()
    }
}

#[test]
fn a_subpage_is_made_below_the_open_page_and_a_taken_name_is_refused() {
    let r = rig(false);
    r.open("handbook.md");
    assert!(r.app.choose(PAGE_SUB));
    assert!(r.modal.submit("Chapter one"), "taken: the dialog stays open");
    assert_eq!(r.paths().len(), 3, "nothing was made");
    assert!(r.modal.submit("Chapter two"));
    assert!(r.paths().contains(&"handbook/chapter-two.md".to_owned()));
    assert_eq!(r.text("handbook/chapter-two.md"), "# Chapter two\n");
    assert!(r.reader.with_reader(|reader| reader.editing().is_some()), "it opens in the editor");
    assert!(!r.modal.no(), "the dialog closed");
}

#[test]
fn bad_names_are_refused_with_a_reason_and_make_nothing() {
    let r = rig(false);
    assert!(r.app.choose(PAGE_NEW));
    for bad in ["   ", "a/b", "?!", "x\u{7}y"] {
        assert!(r.modal.submit(bad), "{bad:?}: the dialog is still open");
    }
    assert_eq!(r.paths().len(), 3);
    assert!(r.modal.no());
}

#[test]
fn rename_moves_the_page_and_its_subpages_keeps_the_history_and_writes_the_title() {
    let r = rig(false);
    let id = r.rt.block_on(r.wiki.get_by_path("handbook.md")).unwrap().unwrap().id;
    let before = r.rt.block_on(r.wiki.history(id, None, 50)).unwrap().items.len();
    r.open("handbook.md");
    assert!(r.app.choose(PAGE_RENAME));
    assert!(r.modal.submit("Notes"), "taken by notes.md: refused, the dialog stays");
    assert!(r.paths().contains(&"handbook.md".to_owned()));
    assert!(r.modal.submit("Manual"));
    assert_eq!(r.paths(), ["manual.md", "manual/chapter-one.md", "notes.md"]);
    assert_eq!(r.text("manual.md"), "# Manual\n\nThe handbook.\n");
    let same = r.rt.block_on(r.wiki.get_by_path("manual.md")).unwrap().unwrap();
    assert_eq!(same.id, id, "the page keeps its id, so its history");
    let after = r.rt.block_on(r.wiki.history(id, None, 50)).unwrap().items.len();
    assert!(after > before, "the move and the new title are in the history ({before} -> {after})");
    assert_eq!(r.reader.with_reader(|reader| reader.current_path().map(str::to_owned)).as_deref(), Some("manual.md"));
}

#[test]
fn delete_asks_first_moves_to_the_trash_and_can_be_restored() {
    let r = rig(true);
    r.open("handbook.md");
    assert!(r.app.choose(PAGE_DELETE));
    assert!(r.modal.no(), "Keep: nothing happens");
    assert_eq!(r.paths().len(), 3);
    assert!(r.app.choose(PAGE_DELETE));
    assert!(r.modal.yes());
    assert_eq!(r.paths(), ["notes.md"], "the page and its subpage left the list");
    let trash = r.rt.block_on(r.wiki.list_trash(None, 10)).unwrap();
    assert_eq!(trash.items.len(), 2, "both are in the trash, not gone");
    let told = matches!(r.status(), Some(Status::Info(t)) if t.contains("Papierkorb"));
    assert!(told, "the answer is German: {:?}", r.status());
    let page = trash.items.iter().find(|i| i.path == "handbook.md").unwrap();
    r.rt.block_on(r.wiki.restore_from_trash(page.id, "")).unwrap();
    assert!(r.paths().contains(&"handbook.md".to_owned()));
}

#[test]
fn discarding_a_new_page_moves_it_to_the_trash_but_an_older_page_stays() {
    let r = rig(false);
    assert!(r.app.choose(PAGE_NEW));
    assert!(r.modal.submit("Scratch"));
    assert!(r.paths().contains(&"scratch.md".to_owned()));
    assert!(r.app.choose(PAGE_DISCARD));
    assert!(r.modal.no());
    assert!(r.reader.with_reader(|reader| reader.editing().is_some()), "kept");
    assert!(r.app.choose(PAGE_DISCARD));
    assert!(r.modal.yes());
    assert!(!r.paths().contains(&"scratch.md".to_owned()), "the never saved page is gone");
    // An existing page: only the edit is discarded.
    r.open("notes.md");
    r.reader.with_reader(|reader| {
        reader.begin_edit();
        reader.set_draft("# Notes\nchanged\n");
    });
    let began = Instant::now();
    while !r.app.choose(PAGE_DISCARD) && began.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(r.modal.yes());
    assert!(r.paths().contains(&"notes.md".to_owned()));
    assert_eq!(r.text("notes.md"), "# Notes\n\nA note.\n");
    assert!(r.reader.with_reader(|reader| reader.editing().is_none()));
}
