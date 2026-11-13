//! The window of the reading wiki: the menu View > Frontend switches between Classic, Confluence and Notion while it runs.
//!
//! Pattern: Strategy in a shell — the reader of `fs-components` is the one surface; a choice in the menu puts the same
//! reader (its state is shared by the clones) into another layout, remembers the choice (`frontend.rs`) and redraws. The
//! menu Page (`window_menu.rs`) and what it does (`window_pages.rs`, dialogs in `window_modal.rs`) belong to the same window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::prelude::text;
use fs_components_app::{AppError, AppWidget, Element, Screen, Status, Surface};
use fs_components_primitives::{FieldSize, SearchField};
use fs_components_widgets::icon::{Glyph, Tone, icon};
use fs_components_workbenches::{ReaderWidget, WikiLabels, styles};

use crate::frontend::{self, Frontend};
use crate::page_words::page_words;
use crate::source::PageMaker;
use crate::ui_texts::UiTexts;
use crate::window_modal::{self, ModalSlot};

pub use crate::window_menu::{
    FRONTEND_PREFIX, PAGE_DELETE, PAGE_DISCARD, PAGE_NEW, PAGE_RENAME, PAGE_SUB, PageState, QUIT, menus, page_menu,
};

/// What the window needs besides the reader.
pub struct Setup {
    /// The words of the layouts (the frontend changes the layout, not the words).
    pub labels: WikiLabels,
    /// Whether the menus speak German.
    pub german: bool,
    /// The switch of the source: a frontend with its own head draws the language bar itself.
    pub bare: Arc<AtomicBool>,
    /// Where the choice is remembered; none = not remembered (a run with `--frontend`).
    pub settings: Option<PathBuf>,
    /// Makes the pages of the menu Page; none = the window has no such menu (a source that cannot make pages).
    pub pages: Option<Arc<dyn PageMaker>>,
}

/// The editor takes the whole width of its column (the layouts of `fs-components` give it a page padding of 40 px and the
/// editor 12 px more, so a 430 px column held a 312 px text area; `max-width` keeps it from running wide in a big window
/// only where the layout set none). Later rules win, so these come after the styles of the workbench. Confluence's own top
/// bar is not drawn: the window has ONE bar at the top (`ReaderSurface::bar`) with the menus, the name and the search.
pub const EDITOR_STYLES: &str = "
.fs-wk-a-page { padding-left: 12px; padding-right: 12px; }
.fs-rd-editor { padding: 0; }
.fs-rd-bar { padding-left: 0; padding-right: 0; }
.fs-wk-a-top { display: none; height: 0px; }
.fs-wiki-bar { display: flex; flex-direction: row; align-items: center; gap: var(--space-4); width: 100%; }
.fs-wiki-brand { display: flex; flex-direction: row; align-items: center; gap: var(--space-2); font-weight: 700; }
";

/// The one surface: the reader, the bar of Confluence and the modal over the page.
pub(crate) struct ReaderSurface {
    pub(crate) widget: Mutex<ReaderWidget>,
    pub(crate) modal: Arc<ModalSlot>,
    confluence: AtomicBool,
    program: String,
    search: SearchField,
}

impl ReaderSurface {
    fn new(widget: ReaderWidget, program: &str, search_hint: &str, shown: Frontend) -> Self {
        let search = {
            let widget = widget.clone();
            SearchField::new(search_hint)
                .size(FieldSize::Small)
                .on_change(Arc::new(move |term| {
                    widget.with_reader(|r| r.search(term));
                    widget.refresh();
                }))
        };
        Self {
            widget: Mutex::new(widget),
            modal: Arc::new(ModalSlot::default()),
            confluence: AtomicBool::new(shown == Frontend::Confluence),
            program: program.to_owned(),
            search,
        }
    }

    pub(crate) fn current(&self) -> ReaderWidget {
        self.widget.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl Surface for ReaderSurface {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        Box::new(self.modal.over(self.current().build(width, height), width, height))
    }

    fn styles(&self) -> String {
        format!("{}{}{EDITOR_STYLES}", styles(), window_modal::STYLES)
    }

    /// Confluence: one line on top with the name, the search and the menus (the shell's title bar and the layout's own top
    /// bar, two lines saying the same name, are not drawn). The other frontends keep the shell's title bar.
    fn bar(&self, menus: Option<Div>, _width: f32) -> Option<(Div, f32)> {
        if !self.confluence.load(Ordering::Relaxed) {
            return None;
        }
        let mut bar = div().class("fs-app-bar").class("fs-wiki-bar").h(BAR_HEIGHT);
        bar = bar
            .child(
                div()
                    .class("fs-wiki-brand")
                    .child(icon(Glyph::FileText, 22.0, Tone::Accent))
                    .child(text(&self.program).no_wrap()),
            )
            .child(div().w(SEARCH_WIDTH).child(self.search.build()));
        if let Some(menus) = menus {
            bar = bar.child(menus);
        }
        Some((bar, BAR_HEIGHT))
    }
}

/// The height of the shell's own bar (`fs-components-app` `BAR_HEIGHT`), so the window loses no room.
const BAR_HEIGHT: f32 = 44.0;
/// The width of the search field in the bar.
const SEARCH_WIDTH: f32 = 360.0;

/// What a choice in the menu does.
pub(crate) struct Switcher {
    pub(crate) app: AppWidget,
    pub(crate) surface: Arc<ReaderSurface>,
    pub(crate) base: ReaderWidget,
    pub(crate) setup: Setup,
    pub(crate) shown: Mutex<Frontend>,
    /// Pages made in this window and not saved since: path -> the revision they had when made.
    pub(crate) fresh: Mutex<HashMap<String, String>>,
    /// The state the menus were last set for.
    menu_state: Mutex<Option<PageState>>,
}

impl Switcher {
    pub(crate) fn texts(&self) -> UiTexts {
        UiTexts::new(self.setup.german)
    }

    /// What decides the entries of the menu Page now.
    fn page_state(&self) -> PageState {
        let (open, editing) = self
            .surface
            .current()
            .with_reader(|r| (r.current_path().is_some(), r.editing().is_some()));
        PageState {
            can_edit: self.setup.pages.as_ref().is_some_and(|p| p.can_create()),
            open,
            editing,
        }
    }

    pub(crate) fn install(self: &Arc<Self>, shown: Frontend) -> Result<(), AppError> {
        let me = Arc::clone(self);
        let mut all = menus(shown, self.setup.german);
        let state = self.page_state();
        if self.setup.pages.is_some() {
            all.insert(1, page_menu(&page_words(self.setup.german), state));
        }
        *self.menu_state.lock().unwrap_or_else(PoisonError::into_inner) = Some(state);
        self.app.set_menus(all, move |id| me.chosen(id))
    }

    /// Set the menus again when what decides them changed (a page was opened, an edit began or ended): the reader changes by
    /// clicks the window does not see, so a small thread of the window looks now and then. A menu that is open is left
    /// alone (setting menus closes it); the next look sets it.
    fn watch_state(self: &Arc<Self>) {
        let me = Arc::downgrade(self);
        std::thread::spawn(move || {
            while let Some(me) = me.upgrade() {
                let open = me.app.with_shell(fs_components_app::Shell::open_menu).is_some();
                let last = *me.menu_state.lock().unwrap_or_else(PoisonError::into_inner);
                if !open && last != Some(me.page_state()) && me.setup.pages.is_some() {
                    let _ = me.install(me.shown());
                }
                drop(me);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });
    }

    fn chosen(self: &Arc<Self>, id: &str) {
        match id {
            QUIT => {
                self.app.quit();
            }
            PAGE_NEW | PAGE_SUB | PAGE_RENAME | PAGE_DELETE | PAGE_DISCARD => self.page_action(id),
            _ => {
                if let Some(chosen) = id.strip_prefix(FRONTEND_PREFIX).and_then(Frontend::parse) {
                    self.show(chosen);
                }
            }
        }
    }

    /// The reader in the layout of `chosen`: the page tree is made again from the list of the source (a new page is in it).
    pub(crate) fn relayout(self: &Arc<Self>, chosen: Frontend) {
        self.setup.bare.store(chosen.has_own_head(), Ordering::Relaxed);
        self.surface
            .confluence
            .store(chosen == Frontend::Confluence, Ordering::Relaxed);
        let widget = self.base.clone().layout(chosen.layout(), self.setup.labels.clone());
        // The page is read again: the plain frontend has the language bar in the text of the page, the others draw it
        // themselves. An edit in progress is left alone.
        widget.with_reader(|r| {
            if r.editing().is_none()
                && let Some(path) = r.current_path().map(str::to_owned)
            {
                r.open(&path);
            }
        });
        *self.surface.widget.lock().unwrap_or_else(PoisonError::into_inner) = widget;
        *self.shown.lock().unwrap_or_else(PoisonError::into_inner) = chosen;
    }

    /// The page tree is made again after every saved edit: the title of the page may be another one now (it is the first
    /// heading). The tree is shared by the clones of the reader widget, so making it on a clone is enough; the save still holds
    /// the reader, so this waits in a thread of its own for the end of the save, then draws the page and the tree again.
    fn watch_saves(self: &Arc<Self>) {
        let Some(pages) = &self.setup.pages else {
            return;
        };
        let me = Arc::downgrade(self);
        pages.on_saved(Arc::new(move || {
            if let Some(me) = me.upgrade() {
                std::thread::spawn(move || {
                    let layout = me.shown().layout();
                    drop(me.base.clone().layout(layout, me.setup.labels.clone()));
                    me.base.refresh();
                });
            }
        }));
    }

    pub(crate) fn shown(&self) -> Frontend {
        *self.shown.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn show(self: &Arc<Self>, chosen: Frontend) {
        self.relayout(chosen);
        if let Some(file) = &self.setup.settings
            && let Err(e) = frontend::save(file, chosen)
        {
            eprintln!("fs-wiki: the frontend is not remembered ({}: {e})", file.display());
            let said = self.texts().plain("msg-frontend-not-kept");
            self.app.set_status(Some(Status::Problem(said)));
        }
        // Setting the menus draws everything again, with the mark on the new frontend.
        if let Err(e) = self.install(chosen) {
            eprintln!("fs-wiki: the menus are not set ({e})");
        }
    }
}

/// The window, not yet opened: the reader in the frontend `shown`, with the menus.
///
/// # Errors
/// The shell cannot be built.
pub fn window(
    title: &str,
    reader: ReaderWidget,
    shown: Frontend,
    setup: Setup,
) -> Result<AppWidget, AppError> {
    window_with_modal(title, reader, shown, setup).map(|(app, _)| app)
}

/// [`window`], and the slot of its modal: what a test (or another part of the program) uses to answer a dialog without a
/// click (`ModalSlot::submit`, `yes`, `no`).
///
/// # Errors
/// The shell cannot be built.
pub fn window_with_modal(
    title: &str,
    reader: ReaderWidget,
    shown: Frontend,
    setup: Setup,
) -> Result<(AppWidget, Arc<ModalSlot>), AppError> {
    setup.bare.store(shown.has_own_head(), Ordering::Relaxed);
    let widget = reader.clone().layout(shown.layout(), setup.labels.clone());
    let hint = reader.with_reader(|r| r.labels().search.clone());
    let surface = Arc::new(ReaderSurface::new(widget, &setup.labels.program, &hint, shown));
    let modal = Arc::clone(&surface.modal);
    let screen = Screen::new("reader", title, Arc::clone(&surface) as Arc<dyn Surface>);
    let app = AppWidget::new(title, vec![screen])?;
    let switcher = Arc::new(Switcher {
        app: app.clone(),
        surface,
        base: reader,
        setup,
        shown: Mutex::new(shown),
        fresh: Mutex::default(),
        menu_state: Mutex::default(),
    });
    switcher.watch_saves();
    switcher.install(shown)?;
    switcher.watch_state();
    Ok((app, modal))
}
