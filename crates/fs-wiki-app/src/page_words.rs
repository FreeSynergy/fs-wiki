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
        CreateError::Taken(path) => t.say("err-taken", &[("name", &name_of(path))]),
        CreateError::Invalid(why) => t.say("err-invalid", &[("why", why)]),
        CreateError::Failed(why) => t.say("err-failed", &[("why", why)]),
        CreateError::NotFound(path) => t.say("err-not-found", &[("path", path)]),
        CreateError::IndexPage => t.plain("err-index"),
        CreateError::Variants => t.plain("err-variants"),
        CreateError::Partial { path, why } => t.say("err-partial", &[("path", path), ("why", why)]),
        CreateError::Half(why) => t.say("err-half", &[("why", why)]),
        CreateError::ReservedName => t.plain("err-reserved"),
    }
}

/// The name a path stands for, for a sentence: its last word without `.md` (the dialog says the typed name instead).
fn name_of(path: &str) -> String {
    let last = path.rsplit('/').next().unwrap_or(path);
    last.strip_suffix(".md").unwrap_or(last).to_owned()
}

/// [`create_error`] where the person typed `name`: a taken name is told with that name, not with the path made from it.
pub fn create_error_named(error: &CreateError, german: bool, name: &str) -> String {
    match error {
        CreateError::Taken(_) => UiTexts::new(german).say("err-taken", &[("name", name)]),
        other => create_error(other, german),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_done_change_is_told_with_where_things_are_and_a_taken_name_with_the_name() {
        let partial = CreateError::Partial {
            path: "new.md".into(),
            why: "conflict".into(),
        };
        assert!(create_error(&partial, false).contains("now at new.md"));
        assert!(create_error(&partial, true).contains("liegt jetzt unter new.md"));
        assert!(create_error(&CreateError::Half("x".into()), false).starts_with("Only part"));
        assert_eq!(
            create_error_named(&CreateError::Taken("a/notes.md".into()), true, "Notes"),
            "Eine Seite „Notes“ gibt es schon."
        );
        assert!(
            !create_error(&CreateError::Failed("w".into()), false).contains("Nothing was changed")
        );
    }
}
