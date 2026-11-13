//! The sentences of the menu Page (new, subpage, rename, delete, discard) and of what it answers, in German and English.
//!
//! Pattern: Adapter over `UiTexts` — the sentences are `MessageFormat` 2 files (`texts/`, ADR 0057); this file only picks
//! the ones the menu and the dialogs need and maps a `CreateError` to its sentence.

use crate::source::CreateError;
use crate::ui_texts::UiTexts;

/// The words of the menu Page and of what it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageWords {
    /// The menu.
    pub menu: String,
    /// The entry that makes a page at the top level.
    pub new_page: String,
    /// The entry that makes a page below the open page.
    pub new_subpage: String,
    /// The entry that gives the open page another name.
    pub rename: String,
    /// The entry that deletes the open page.
    pub delete: String,
    /// The entry that discards the draft in the editor.
    pub discard: String,
    /// The entries, when the window cannot edit: the reason.
    pub read_only: String,
    /// The entries, when no page is open: the reason.
    pub no_page: String,
    /// The entry that discards, when nothing is being edited: the reason.
    pub no_draft: String,
    /// The entries that change a page, while one is being edited: the reason.
    pub editing: String,
    /// The entries, for a page with language variants: the reason.
    pub variants: String,
    /// The title of a new page when a step makes one without a name.
    pub title: String,
    /// A subpage was asked for and no page is open.
    pub need_page: String,
    /// A page was asked for while the person has unsaved changes.
    pub finish_edit: String,
}

/// The words of the interface language.
pub fn page_words(german: bool) -> PageWords {
    let t = UiTexts::new(german);
    PageWords {
        menu: t.plain("menu-page"),
        new_page: t.plain("page-new"),
        new_subpage: t.plain("page-subpage"),
        rename: t.plain("page-rename"),
        delete: t.plain("page-delete"),
        discard: t.plain("page-discard"),
        read_only: t.plain("reason-read-only"),
        no_page: t.plain("reason-no-page"),
        no_draft: t.plain("reason-no-draft"),
        editing: t.plain("reason-editing"),
        variants: t.plain("reason-variants"),
        title: t.plain("page-new"),
        need_page: t.plain("msg-need-page"),
        finish_edit: t.plain("msg-finish-edit"),
    }
}

/// What a person is told when a page could not be made, renamed or deleted.
pub fn create_error(error: &CreateError, german: bool) -> String {
    let t = UiTexts::new(german);
    match error {
        CreateError::ReadOnly => t.plain("err-read-only"),
        CreateError::EmptyTitle => t.plain("err-empty"),
        CreateError::NoLetter => t.plain("err-no-letter"),
        CreateError::BadChars => t.plain("err-chars"),
        CreateError::Taken(path) => t.say("err-taken", &[("path", path)]),
        CreateError::Invalid(why) => t.say("err-invalid", &[("why", why)]),
        CreateError::Failed(why) => t.say("err-failed", &[("why", why)]),
        CreateError::NotFound(path) => t.say("err-not-found", &[("path", path)]),
        CreateError::IndexPage => t.plain("err-index"),
        CreateError::Variants => t.plain("err-variants"),
        CreateError::Partial(why) => t.say("err-partial", &[("why", why)]),
    }
}
