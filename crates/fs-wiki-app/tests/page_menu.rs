//! The menu Page of the wiki window: which entries can be chosen when, and what it says when one cannot.
//!
//! Pattern: Integration test over the public API; a small wiki in memory is the source and the maker of pages, an entry is
//! chosen as a press on its line does (`choose`), the status line and the reader are read afterwards.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use fs_components_app::{AppWidget, Status};
use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, EditableText, Entry, Hit, Labels, LinkTarget, Reader, ReaderWidget,
    SourceError, WikiLabels,
};
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{
    PAGE_DELETE, PAGE_DISCARD, PAGE_NEW, PAGE_RENAME, PAGE_SUB, Setup, window_with_modal,
};
use fs_wiki_app::window_modal::ModalSlot;
use fs_wiki_app::source::{CreateError, PageMaker, create::page_path};
use std::time::{Duration, Instant};

/// A wiki in memory: (path, text); a name that is taken is refused.
struct Mem {
    pages: Mutex<Vec<(String, String)>>,
    editable: bool,
}

impl DocumentSource for Mem {
    fn entries(&self, _prefix: &str) -> Result<Vec<Entry>, SourceError> {
        let pages = self.pages.lock().unwrap();
        Ok(pages
            .iter()
            .map(|(p, t)| Entry::new(p, t.trim_start_matches("# ").trim()))
            .collect())
    }
    fn read(&self, path: &str) -> Result<Document, SourceError> {
        Ok(Document::new(path, Vec::new()))
    }
    fn resolve(&self, _from: &str, href: &str) -> LinkTarget {
        LinkTarget::Missing(href.into())
    }
    fn find(&self, _term: &str) -> Result<Vec<Hit>, SourceError> {
        Ok(Vec::new())
    }
    fn can_edit(&self, _path: &str) -> bool {
        self.editable
    }
    fn source_text(&self, path: &str) -> Result<EditableText, SourceError> {
        let pages = self.pages.lock().unwrap();
        let (_, text) = pages
            .iter()
            .find(|(p, _)| p == path)
            .ok_or_else(|| SourceError::not_found(path))?;
        Ok(EditableText::new(text.clone(), None))
    }
}

impl PageMaker for Mem {
    fn can_create(&self) -> bool {
        self.editable
    }
    fn create_page(&self, parent: Option<&str>, title: &str) -> Result<String, CreateError> {
        self.create_exact(parent, title)
    }
    fn create_exact(&self, parent: Option<&str>, title: &str) -> Result<String, CreateError> {
        let path = page_path(parent, title).ok_or(CreateError::NoLetter)?;
        let mut pages = self.pages.lock().unwrap();
        if pages.iter().any(|(p, _)| *p == path) {
            return Err(CreateError::Taken(path));
        }
        pages.push((path.clone(), format!("# {title}\n")));
        Ok(path)
    }
}

fn app(editable: bool, german: bool) -> (AppWidget, ReaderWidget, Arc<Mem>, Arc<ModalSlot>) {
    let mem = Arc::new(Mem {
        pages: Mutex::new(vec![("home.md".into(), "# Home\n".into())]),
        editable,
    });
    let reader = ReaderWidget::new(mem.clone(), Labels::default(), None);
    let setup = Setup {
        labels: WikiLabels::default(),
        german,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(mem.clone()),
    };
    let (app, modal) =
        window_with_modal("Wiki", reader.clone(), Frontend::Confluence, setup).expect("window");
    (app, reader, mem, modal)
}

fn status(app: &AppWidget) -> Option<Status> {
    app.with_shell(|s| s.status().cloned())
}

/// (id, label, disabled) of the entries of the menu Page.
fn page_menu(app: &AppWidget) -> Vec<(String, String, bool)> {
    app.with_shell(|s| {
        s.menus()
            .iter()
            .find(|m| m.label == "Page" || m.label == "Seite")
            .map(|m| {
                m.entries
                    .iter()
                    .map(|e| (e.id.clone(), e.label.clone(), e.disabled))
                    .collect()
            })
            .unwrap_or_default()
    })
}

fn locked(app: &AppWidget) -> Vec<String> {
    page_menu(app)
        .into_iter()
        .filter(|(_, _, d)| *d)
        .map(|(id, ..)| id)
        .collect()
}

/// The window looks at the reader now and then (it cannot see a click in the tree): wait for the menu to be what it should be.
fn wait_for(app: &AppWidget, want: &[&str]) -> Vec<String> {
    let began = Instant::now();
    loop {
        let now = locked(app);
        if now == want || began.elapsed() > Duration::from_secs(5) {
            return now;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn the_menu_has_five_entries_in_both_languages_and_nothing_open_locks_four() {
    let (en, ..) = app(true, false);
    let ids: Vec<String> = page_menu(&en).into_iter().map(|(id, ..)| id).collect();
    assert_eq!(ids, [PAGE_NEW, PAGE_SUB, PAGE_RENAME, PAGE_DELETE, PAGE_DISCARD]);
    assert_eq!(page_menu(&en)[0].1, "New page");
    assert_eq!(page_menu(&en)[1].1, "New subpage (no page is open)");
    assert_eq!(locked(&en), [PAGE_SUB, PAGE_RENAME, PAGE_DELETE, PAGE_DISCARD]);
    let (de, ..) = app(true, true);
    assert_eq!(page_menu(&de)[0].1, "Neue Seite");
    assert_eq!(page_menu(&de)[1].1, "Neue Unterseite (keine Seite offen)");
    assert_eq!(page_menu(&de)[3].1, "Löschen (keine Seite offen)");
}

#[test]
fn a_window_that_cannot_edit_shows_every_entry_locked_with_the_reason() {
    let (app, _, _, modal) = app(false, false);
    for (_, label, disabled) in page_menu(&app) {
        assert!(disabled, "{label}");
        assert!(label.contains("read only"), "{label}");
    }
    assert!(!app.choose(PAGE_NEW), "a locked entry does nothing");
    assert!(!modal.no(), "no dialog was opened");
}

#[test]
fn the_entries_follow_what_is_open_and_what_is_edited() {
    let (app, reader, ..) = app(true, false);
    reader.with_reader(|r| r.open("home.md"));
    assert_eq!(wait_for(&app, &[PAGE_DISCARD]), [PAGE_DISCARD], "a page is open: all but Discard");
    reader.with_reader(Reader::begin_edit);
    assert_eq!(
        wait_for(&app, &[PAGE_NEW, PAGE_SUB, PAGE_RENAME, PAGE_DELETE]),
        [PAGE_NEW, PAGE_SUB, PAGE_RENAME, PAGE_DELETE],
        "being edited: only Discard"
    );
    assert!(page_menu(&app)[2].1.contains("finish the edit first"));
    reader.with_reader(|r| {
        r.cancel_edit();
    });
    assert_eq!(wait_for(&app, &[PAGE_DISCARD]), [PAGE_DISCARD]);
}

#[test]
fn a_name_makes_the_page_and_a_refused_name_stays_in_its_dialog() {
    let (app, reader, mem, modal) = app(true, false);
    assert!(app.choose(PAGE_NEW));
    assert!(modal.submit("   "), "a blank name is answered, not taken");
    assert_eq!(mem.pages.lock().unwrap().len(), 1, "nothing was made");
    assert!(modal.submit("Home"), "the dialog is still open: the name is taken");
    assert_eq!(mem.pages.lock().unwrap().len(), 1);
    assert!(modal.submit("Guide"));
    assert!(!modal.submit("again"), "the dialog closed after the page was made");
    let (path, editing) = reader.with_reader(|r| {
        (r.current_path().map(str::to_owned), r.editing().map(|e| e.draft.clone()))
    });
    assert_eq!(path.as_deref(), Some("guide.md"));
    assert_eq!(editing.as_deref(), Some("# Guide\n"));
    assert!(matches!(status(&app), Some(Status::Info(t)) if t.contains("guide.md")));
    // While it is edited another page waits: the entry is locked at once.
    assert!(!app.choose(PAGE_NEW));
    assert!(locked(&app).contains(&PAGE_NEW.to_string()));
}

#[test]
fn a_subpage_goes_below_the_open_page_and_escape_leaves() {
    let (app, reader, mem, modal) = app(true, false);
    reader.with_reader(|r| r.open("home.md"));
    wait_for(&app, &[PAGE_DISCARD]);
    assert!(app.choose(PAGE_SUB));
    assert!(modal.no(), "Escape / Cancel closes it");
    assert!(!modal.no(), "nothing is open any more");
    assert_eq!(mem.pages.lock().unwrap().len(), 1);
    assert!(app.choose(PAGE_SUB));
    assert!(modal.submit("Part"));
    let paths: Vec<String> = mem.pages.lock().unwrap().iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(paths, ["home.md", "home/part.md"]);
}
