//! The surface of the wiki window: the reader, the one bar of Confluence and the styles of the app.
//!
//! Pattern: Decorator — the reader widget of `fs-components` is drawn as it is; this surface adds what the window needs
//! around it (the bar with the name, the search and the menus in Confluence; the styles that override the layouts' paddings).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::prelude::text;
use fs_components_app::{Element, Surface};
use fs_components_primitives::{FieldSize, SearchField};
use fs_components_widgets::icon::{Glyph, Tone, icon};
use fs_components_workbenches::{ReaderWidget, styles};

use crate::frontend::Frontend;
use crate::window_modal::{self, ModalSlot};

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
    pub(crate) confluence: AtomicBool,
    program: String,
    search: SearchField,
}

impl ReaderSurface {
    pub(crate) fn new(
        widget: ReaderWidget,
        program: &str,
        search_hint: &str,
        shown: Frontend,
    ) -> Self {
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
        self.widget
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Surface for ReaderSurface {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        Box::new(self.current().build(width, height))
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

/// The height of the one bar: the search field (small) needs 44 px and a margin; the two bars it replaces were 44 + 52.
const BAR_HEIGHT: f32 = 52.0;
/// The width of the search field in the bar.
const SEARCH_WIDTH: f32 = 360.0;
