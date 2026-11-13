//! The window of the reading wiki: the menu View > Face switches between Classic, Confluence and Notion while it runs.
//!
//! Pattern: Strategy in a shell — the reader of `fs-components` is the one surface; a choice in the menu puts the same
//! reader (its state is shared by the clones) into another layout, remembers the choice (`face.rs`) and redraws.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components_app::{AppError, AppWidget, Element, Menu, MenuEntry, Screen, Status, Surface};
use fs_components_workbenches::{ReaderWidget, WikiLabels, styles};

use crate::face::{self, Face};

/// Id of the entry that ends the program.
pub const QUIT: &str = "program-quit";
/// Prefix of the ids of the entries that choose a face (`face-classic`, `face-a`, `face-b`).
pub const FACE_PREFIX: &str = "face-";

/// What the window needs besides the reader.
pub struct Setup {
    /// The words of the layouts (the face changes the layout, not the words).
    pub labels: WikiLabels,
    /// Whether the menus speak German.
    pub german: bool,
    /// The switch of the source: a face with its own head draws the language bar itself.
    pub bare: Arc<AtomicBool>,
    /// Where the choice is remembered; none = not remembered (a run with `--frontend`).
    pub settings: Option<PathBuf>,
}

/// The menus: Program (Quit) and View with one line per face, the shown one marked.
pub fn menus(shown: Face, german: bool) -> Vec<Menu> {
    let (program, quit, view, face_word) = if german {
        ("Programm", "Beenden", "Ansicht", "Gesicht")
    } else {
        ("Program", "Quit", "View", "Face")
    };
    let entries = Face::ALL
        .into_iter()
        .map(|f| {
            let name = match f {
                Face::Classic => "Classic",
                Face::Confluence => "Confluence",
                Face::Notion => "Notion",
            };
            let mark = if f == shown { "✓ " } else { "   " };
            MenuEntry::new(
                format!("{FACE_PREFIX}{}", f.id()),
                format!("{mark}{face_word}: {name}"),
            )
        })
        .collect();
    vec![
        Menu::new(program, vec![MenuEntry::new(QUIT, quit)]),
        Menu::new(view, entries),
    ]
}

struct ReaderSurface(Mutex<ReaderWidget>);

impl Surface for ReaderSurface {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        let widget = self.0.lock().unwrap_or_else(PoisonError::into_inner).clone();
        Box::new(widget.build(width, height))
    }

    fn styles(&self) -> String {
        styles()
    }
}

/// What a choice in the menu does.
struct Switcher {
    app: AppWidget,
    surface: Arc<ReaderSurface>,
    base: ReaderWidget,
    setup: Setup,
}

impl Switcher {
    fn install(self: &Arc<Self>, shown: Face) -> Result<(), AppError> {
        let me = Arc::clone(self);
        self.app
            .set_menus(menus(shown, self.setup.german), move |id| me.chosen(id))
    }

    fn chosen(self: &Arc<Self>, id: &str) {
        if id == QUIT {
            self.app.quit();
        } else if let Some(face) = id.strip_prefix(FACE_PREFIX).and_then(Face::parse) {
            self.show(face);
        }
    }

    fn show(self: &Arc<Self>, face: Face) {
        self.setup.bare.store(face.has_own_head(), Ordering::Relaxed);
        let widget = self
            .base
            .clone()
            .layout(face.layout(), self.setup.labels.clone());
        // The page is read again: the plain face has the language bar in the text of the page, the others draw it
        // themselves. An edit in progress is left alone.
        widget.with_reader(|r| {
            if r.editing().is_none()
                && let Some(path) = r.current_path().map(str::to_owned)
            {
                r.open(&path);
            }
        });
        *self.surface.0.lock().unwrap_or_else(PoisonError::into_inner) = widget;
        if let Some(file) = &self.setup.settings
            && let Err(e) = face::save(file, face)
        {
            eprintln!("fs-wiki: the face is not remembered ({}: {e})", file.display());
            let words = if self.setup.german {
                "Das Gesicht konnte nicht gemerkt werden."
            } else {
                "The face could not be remembered."
            };
            self.app.set_status(Some(Status::Problem(words.into())));
        }
        // Setting the menus draws everything again, with the mark on the new face.
        if let Err(e) = self.install(face) {
            eprintln!("fs-wiki: the menus are not set ({e})");
        }
    }
}

/// The window, not yet opened: the reader in the face `face`, with the menus.
///
/// # Errors
/// The shell cannot be built.
pub fn window(
    title: &str,
    reader: ReaderWidget,
    face: Face,
    setup: Setup,
) -> Result<AppWidget, AppError> {
    setup.bare.store(face.has_own_head(), Ordering::Relaxed);
    let widget = reader
        .clone()
        .layout(face.layout(), setup.labels.clone());
    let surface = Arc::new(ReaderSurface(Mutex::new(widget)));
    let screen = Screen::new("reader", title, Arc::clone(&surface) as Arc<dyn Surface>);
    let app = AppWidget::new(title, vec![screen])?;
    let switcher = Arc::new(Switcher {
        app: app.clone(),
        surface,
        base: reader,
        setup,
    });
    switcher.install(face)?;
    Ok(app)
}
