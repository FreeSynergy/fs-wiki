//! The sentences of the menu Page (new page, new subpage) in German and English.
//!
//! Pattern: Dictionary — one table per language, a sentence is whole with named places (`{path}`, `{why}`), never built from
//! pieces. The words of the layouts are in `frontend.rs`, the others of the app in `words.rs`; the central unification of
//! the interface language will replace these tables, not the code that asks them.

use crate::source::CreateError;
use crate::words::say;

/// The words of the menu Page and of what it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageWords {
    /// The menu.
    pub menu: &'static str,
    /// The entry that makes a page at the top level.
    pub new_page: &'static str,
    /// The entry that makes a page below the open page.
    pub new_subpage: &'static str,
    /// The entries, when the window cannot edit: the reason.
    pub read_only: &'static str,
    /// The title of a new page until the person writes another one.
    pub title: &'static str,
    /// A subpage was asked for and no page is open.
    pub need_page: &'static str,
    /// A page was asked for while the person has unsaved changes.
    pub finish_edit: &'static str,
    /// `{path}`: the page was made.
    pub created: &'static str,
}

/// The words of the interface language.
pub fn page_words(german: bool) -> PageWords {
    if german {
        PageWords {
            menu: "Seite",
            new_page: "Neue Seite",
            new_subpage: "Neue Unterseite",
            read_only: "nur lesen: dieses Wiki wird hier nicht bearbeitet",
            title: "Neue Seite",
            need_page: "Zuerst eine Seite öffnen: die neue Seite wird ihre Unterseite.",
            finish_edit: "Erst die Änderungen speichern oder verwerfen, dann eine neue Seite anlegen.",
            created: "Seite angelegt ({path}). Titel und Text schreiben, dann speichern (Strg+S).",
        }
    } else {
        PageWords {
            menu: "Page",
            new_page: "New page",
            new_subpage: "New subpage",
            read_only: "read only: this wiki is not edited here",
            title: "New page",
            need_page: "Open a page first: the new page becomes its subpage.",
            finish_edit: "Save or discard the changes first, then make a new page.",
            created: "Page made ({path}). Write the title and the text, then save (Ctrl+S).",
        }
    }
}

/// What a person is told when a page could not be made.
pub fn create_error(error: &CreateError, german: bool) -> String {
    let (read_only, empty, taken, invalid, failed) = if german {
        (
            "Das Wiki ist hier nur lesbar; die Seite wurde nicht angelegt.",
            "Der Titel enthält keinen Buchstaben und keine Ziffer; daraus lässt sich kein Pfad machen.",
            "Der Pfad {path} und seine nummerierten Nachfolger sind vergeben; die Seite wurde nicht angelegt.",
            "Der Pfad ist nicht erlaubt ({why}); die Seite wurde nicht angelegt.",
            "Die Seite wurde nicht angelegt: {why}",
        )
    } else {
        (
            "The wiki is read only here; no page was made.",
            "The title has no letter or digit to make a path from.",
            "The path {path} and its numbered successors are taken; no page was made.",
            "The path is not allowed ({why}); no page was made.",
            "No page was made: {why}",
        )
    };
    match error {
        CreateError::ReadOnly => read_only.to_owned(),
        CreateError::EmptyTitle => empty.to_owned(),
        CreateError::Taken(path) => say(taken, &[("path", path)]),
        CreateError::Invalid(why) => say(invalid, &[("why", why)]),
        CreateError::Failed(why) => say(failed, &[("why", why)]),
    }
}
