//! The menus of the wiki window: Program, Page (by what is open and what is being edited) and View.
//!
//! Pattern: State — the menu Page is a pure function of what the window is doing (`PageState`): an entry that makes no sense
//! now is shown locked with the reason in brackets, never hidden (a person finds it and learns when it works).

use fs_components_app::{Menu, MenuEntry};

use crate::frontend::Frontend;
use crate::page_words::PageWords;
use crate::ui_texts::UiTexts;

/// Id of the entry that ends the program.
pub const QUIT: &str = "program-quit";
/// Prefix of the ids of the entries that choose a frontend (`frontend-classic`, `frontend-a`, `frontend-b`).
pub const FRONTEND_PREFIX: &str = "frontend-";
/// Id of the entry that makes a page at the top level.
pub const PAGE_NEW: &str = "page-new";
/// Id of the entry that makes a page below the open page.
pub const PAGE_SUB: &str = "page-subpage";
/// Id of the entry that gives the open page another name.
pub const PAGE_RENAME: &str = "page-rename";
/// Id of the entry that deletes the open page.
pub const PAGE_DELETE: &str = "page-delete";
/// Id of the entry that discards the draft in the editor.
pub const PAGE_DISCARD: &str = "page-discard";

/// What decides which entries of the menu Page can be chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageState {
    /// The window can change pages (not read only).
    pub can_edit: bool,
    /// A page is open.
    pub open: bool,
    /// The open page is in the editor.
    pub editing: bool,
}

/// The menu Page for `state`.
pub fn page_menu(words: &PageWords, state: PageState) -> Menu {
    let entry = |id: &str, name: &str, reason: Option<&str>| match reason {
        None => MenuEntry::new(id, name),
        Some(why) => MenuEntry::new(id, format!("{name} ({why})")).disabled(),
    };
    let read_only = (!state.can_edit).then_some(words.read_only.as_str());
    let editing = state.editing.then_some(words.editing.as_str());
    let no_page = (!state.open).then_some(words.no_page.as_str());
    let before_edit = read_only.or(editing);
    let on_page = before_edit.or(no_page);
    let discard = read_only.or((!state.editing).then_some(words.no_draft.as_str()));
    Menu::new(
        words.menu.clone(),
        vec![
            entry(PAGE_NEW, &words.new_page, before_edit),
            entry(PAGE_SUB, &words.new_subpage, on_page),
            entry(PAGE_RENAME, &words.rename, on_page),
            entry(PAGE_DELETE, &words.delete, on_page),
            entry(PAGE_DISCARD, &words.discard, discard),
        ],
    )
}

/// The menus Program (Quit) and View with one line per frontend, the shown one marked, in the interface language.
pub fn menus(shown: Frontend, german: bool) -> Vec<Menu> {
    let t = UiTexts::new(german);
    let entries = Frontend::ALL
        .into_iter()
        .map(|f| {
            let name = match f {
                Frontend::Classic => "Classic",
                Frontend::Confluence => "Confluence",
                Frontend::Notion => "Notion",
            };
            let mark = if f == shown { "✓ " } else { "   " };
            MenuEntry::new(
                format!("{FRONTEND_PREFIX}{}", f.id()),
                format!("{mark}{}", t.say("menu-frontend", &[("name", name)])),
            )
        })
        .collect();
    vec![
        Menu::new(t.plain("menu-program"), vec![MenuEntry::new(QUIT, t.plain("menu-quit"))]),
        Menu::new(t.plain("menu-view"), entries),
    ]
}
