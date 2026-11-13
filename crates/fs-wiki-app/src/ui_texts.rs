//! The words of the wiki window, from `MessageFormat` 2 files (ADR 0057): menus, dialogs, answers, the words of the layouts
//! and of the reader, and those of the Admin window.
//!
//! Pattern: Factory over embedded `texts/en.mf2` and `texts/de.mf2` — one `UiTexts` per interface language; a sentence is
//! whole with named holes (`{$path}`), never built from pieces. `check_pairs(TEXTS)` must be empty (a test asserts it), so
//! no id exists in one language only. The words the older tables (`lang_words.rs`, `words.rs`) hold are to follow.

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_widgets::TitleLabels;
use fs_components_words::{Args, Locale, Localized, TextId, Words};
use fs_components_workbenches::{Labels, WikiLabels};

/// The two language files of the window.
pub const TEXTS: &[Source] = &[
    Source {
        language: "en",
        text: include_str!("texts/en.mf2"),
    },
    Source {
        language: "de",
        text: include_str!("texts/de.mf2"),
    },
];

/// The words of one interface language.
pub struct UiTexts {
    texts: Mf2Texts,
    locale: Locale,
}

impl UiTexts {
    /// German or English.
    pub fn new(german: bool) -> Self {
        let locale =
            Locale::parse(if german { "de" } else { "en" }).unwrap_or_else(|_| Locale::english());
        Self {
            texts: Mf2Texts::new(TEXTS, &locale),
            locale,
        }
    }

    /// The language these words are in.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// The sentence `id` without holes.
    pub fn plain(&self, id: &str) -> String {
        self.texts.plain(id)
    }

    /// The sentence `id`, its holes filled with `values` (`(name, text)`).
    pub fn say(&self, id: &str, values: &[(&str, &str)]) -> String {
        let args = values
            .iter()
            .fold(Args::new(), |args, (name, value)| args.text(name, *value));
        self.texts.say(&TextId::new(id), &args).text().to_owned()
    }

    /// The sentence `id` with the two characters `{}` in the place of its one hole: the form `fs-components` fills itself.
    fn holey(&self, id: &str) -> String {
        self.say(id, &[("hole", "{}")])
    }

    /// The words of the wiki layouts.
    pub fn wiki_labels(&self) -> WikiLabels {
        let mut w = WikiLabels::default();
        w.program = self.plain("wk-program");
        w.pages = self.plain("wk-pages");
        w.filter = self.plain("wk-filter");
        w.outline = self.plain("wk-outline");
        w.no_headings = self.plain("wk-no-headings");
        w.history = self.plain("wk-history");
        w.no_history = self.plain("wk-no-history");
        w.state_current = self.plain("wk-state-current");
        w.state_outdated = self.plain("wk-state-outdated");
        w.state_fallback = self.plain("wk-state-fallback");
        w.language = self.plain("wk-language");
        w.edit = self.plain("wk-edit");
        w.share = self.plain("wk-share");
        w.block_hint = self.plain("wk-block-hint");
        w.title_hint = TitleLabels::for_locale(&self.locale).placeholder;
        w.page_count = self.holey("wk-page-count");
        w
    }

    /// The words of the reader (the plain frontend, the editor, the notices).
    pub fn reader_labels(&self) -> Labels {
        let mut l = Labels::default();
        for (field, id) in [
            (&mut l.search, "rd-search"),
            (&mut l.back, "rd-back"),
            (&mut l.edit, "rd-edit"),
            (&mut l.save, "rd-save"),
            (&mut l.cancel, "rd-cancel"),
            (&mut l.export, "rd-export"),
            (&mut l.nothing_open, "rd-nothing-open"),
            (&mut l.saved, "rd-saved"),
            (&mut l.no_changes, "rd-no-changes"),
            (&mut l.unsaved, "rd-unsaved"),
            (&mut l.discard_confirm, "rd-discard-confirm"),
            (&mut l.backup_found, "rd-backup-found"),
            (&mut l.restore, "rd-restore"),
            (&mut l.discard, "rd-discard"),
            (&mut l.backup_stale, "rd-backup-stale"),
            (&mut l.restore_confirm, "rd-restore-confirm"),
        ] {
            *field = self.plain(id);
        }
        for (field, id) in [
            (&mut l.nothing_found, "rd-nothing-found"),
            (&mut l.unreadable, "rd-unreadable"),
            (&mut l.missing_target, "rd-missing-target"),
            (&mut l.not_saved, "rd-not-saved"),
            (&mut l.cannot_edit, "rd-cannot-edit"),
            (&mut l.not_exported, "rd-not-exported"),
            (&mut l.overwrite_confirm, "rd-overwrite-confirm"),
            (&mut l.lossy, "rd-lossy"),
            (&mut l.conflict_confirm, "rd-conflict-confirm"),
            (&mut l.backup_mismatch, "rd-backup-mismatch"),
            (&mut l.backup_failed, "rd-backup-failed"),
            (&mut l.glyphs_missing, "rd-glyphs-missing"),
            (&mut l.glyphs_titles, "rd-glyphs-titles"),
            (&mut l.glyphs_more, "rd-glyphs-more"),
            (&mut l.list_failed, "rd-list-failed"),
            (&mut l.search_failed, "rd-search-failed"),
        ] {
            *field = self.holey(id);
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_is_in_german_and_english_with_the_same_holes() {
        assert_eq!(fs_components_mf2::check_pairs(TEXTS), Vec::<String>::new());
    }

    #[test]
    fn the_layout_and_reader_words_follow_the_language() {
        let (de, en) = (UiTexts::new(true), UiTexts::new(false));
        assert_eq!(de.wiki_labels().pages, "Seiten");
        assert_eq!(en.wiki_labels().pages, "Pages");
        assert_eq!(de.wiki_labels().page_count, "{} Seiten");
        assert_eq!(en.reader_labels().save, "Save");
        assert_eq!(de.reader_labels().save, "Speichern");
        assert_eq!(
            de.reader_labels().nothing_found,
            "Nichts gefunden für „{}“."
        );
        assert_eq!(de.wiki_labels().title_hint, "Ohne Titel");
    }

    #[test]
    fn a_hole_is_filled_and_no_word_is_missing() {
        let de = UiTexts::new(true);
        assert_eq!(
            de.say("err-taken", &[("path", "a.md")]),
            "Unter a.md gibt es schon eine Seite."
        );
        // A missing id would come back as itself; none of the ids the window asks for may.
        let ids = [
            "menu-page",
            "page-new",
            "dlg-keep",
            "err-empty",
            "wk-title-hint",
            "admin-type-trash",
        ];
        for id in ids {
            for ui in [&de, &UiTexts::new(false)] {
                assert_ne!(ui.plain(id), id, "{id}");
            }
        }
    }
}
