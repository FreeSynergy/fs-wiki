//! The menu Page of the wiki window: new page, new subpage; what it does and what it says when it cannot.
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
use fs_wiki_app::frontend_window::{PAGE_NEW, PAGE_SUB, Setup, window};
use fs_wiki_app::source::{CreateError, PageMaker, create::page_path};

/// A wiki in memory: (path, text) and the answer the next `create_page` gives instead of making a page.
struct Mem {
    pages: Mutex<Vec<(String, String)>>,
    refuse: Mutex<Option<CreateError>>,
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
        if let Some(e) = self.refuse.lock().unwrap().take() {
            return Err(e);
        }
        let path = page_path(parent, title).ok_or(CreateError::EmptyTitle)?;
        self.pages
            .lock()
            .unwrap()
            .push((path.clone(), format!("# {title}\n")));
        Ok(path)
    }
}

fn app(editable: bool, german: bool) -> (AppWidget, ReaderWidget, Arc<Mem>) {
    app_with(&[("home.md", "# Home\n")], editable, german)
}

fn app_with(
    pages: &[(&str, &str)],
    editable: bool,
    german: bool,
) -> (AppWidget, ReaderWidget, Arc<Mem>) {
    let mem = Arc::new(Mem {
        pages: Mutex::new(
            pages
                .iter()
                .map(|(p, t)| ((*p).into(), (*t).into()))
                .collect(),
        ),
        refuse: Mutex::new(None),
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
    let app = window("Wiki", reader.clone(), Frontend::Confluence, setup).expect("window");
    (app, reader, mem)
}

fn status(app: &AppWidget) -> Option<Status> {
    app.with_shell(|s| s.status().cloned())
}

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

#[test]
fn the_menu_has_new_page_and_new_subpage_in_both_languages() {
    let (en, ..) = app(true, false);
    assert_eq!(
        page_menu(&en),
        [
            (PAGE_NEW.into(), "New page".into(), false),
            (PAGE_SUB.into(), "New subpage".into(), false)
        ]
    );
    let (de, ..) = app(true, true);
    assert_eq!(page_menu(&de)[0].1, "Neue Seite");
    assert_eq!(page_menu(&de)[1].1, "Neue Unterseite");
}

#[test]
fn a_window_that_cannot_edit_shows_the_entries_locked_with_the_reason() {
    let (app, ..) = app(false, false);
    for (_, label, disabled) in page_menu(&app) {
        assert!(disabled, "{label}");
        assert!(label.contains("read only"), "{label}");
    }
    assert!(!app.choose(PAGE_NEW), "a locked entry does nothing");
}

#[test]
fn a_new_page_opens_in_the_editor_and_a_subpage_goes_below_the_open_page() {
    let (app, reader, mem) = app(true, false);
    assert!(app.choose(PAGE_NEW));
    let (path, editing) = reader.with_reader(|r| {
        (
            r.current_path().map(str::to_owned),
            r.editing().map(|e| e.draft.clone()),
        )
    });
    assert_eq!(path.as_deref(), Some("new-page.md"));
    assert_eq!(editing.as_deref(), Some("# New page\n"));
    assert!(matches!(status(&app), Some(Status::Info(t)) if t.contains("new-page.md")));
    assert!(
        reader.with_reader(|r| r.entries().iter().any(|e| e.path == "new-page.md")),
        "in the list"
    );
    assert!(app.choose(PAGE_SUB));
    let paths: Vec<String> = mem
        .pages
        .lock()
        .unwrap()
        .iter()
        .map(|(p, _)| p.clone())
        .collect();
    assert_eq!(paths, ["home.md", "new-page.md", "new-page/new-page.md"]);
}

#[test]
fn what_cannot_be_done_is_said_in_words_not_by_a_dead_button() {
    // A subpage in an empty wiki: no page is open to be its parent.
    let (empty, _, none) = app_with(&[], true, false);
    assert!(empty.choose(PAGE_SUB));
    assert!(matches!(status(&empty), Some(Status::Problem(t)) if t.contains("Open a page first")));
    assert!(none.pages.lock().unwrap().is_empty(), "nothing was made");
    let (app, reader, mem) = app(true, false);
    // Unsaved changes are not left behind.
    reader.with_reader(|r| {
        r.open("home.md");
        r.begin_edit();
        r.set_draft("# Home\nchanged\n");
    });
    assert!(app.choose(PAGE_NEW));
    assert!(matches!(status(&app), Some(Status::Problem(t)) if t.contains("Save or discard")));
    assert_eq!(mem.pages.lock().unwrap().len(), 1);
    // A refusal of the wiki names its reason (here: the path is taken).
    reader.with_reader(Reader::cancel_edit);
    reader.with_reader(Reader::cancel_edit);
    *mem.refuse.lock().unwrap() = Some(CreateError::Taken("new-page.md".into()));
    assert!(app.choose(PAGE_NEW));
    assert!(
        matches!(status(&app), Some(Status::Problem(t)) if t.contains("new-page.md") && t.contains("taken"))
    );
}
