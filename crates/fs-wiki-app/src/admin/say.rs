//! The words of the Admin frontend for this program, until translation exists (ADR 0053).
//!
//! Pattern: Strategy behind `Say` — a translation key goes in, the words of the interface language come out. The words of
//! the frontend itself are in the `MessageFormat` 2 files of the app (`texts/`, ids `admin-<key with dashes>`); for the keys of this program's descriptions (`wiki.<type>.<part>`, `wiki.<type>.command.<name>`) the last
//! part is spelled out (`redirect-to` becomes "Redirect to"), because the descriptions only ever carry such keys. An
//! unknown key comes back as itself, so a missing word is seen, not hidden.

use fs_components_fields::Say;

use crate::ui_texts::UiTexts;

fn spelled(part: &str) -> String {
    let mut s = part.replace('-', " ");
    if let Some(first) = s.get(..1) {
        s = first.to_uppercase() + &s[1..];
    }
    s
}

/// The words, in German or English (English by default).
#[derive(Clone, Copy, Debug, Default)]
pub struct Words {
    german: bool,
}

impl Words {
    /// The words of the interface language.
    pub fn new(german: bool) -> Self {
        Self { german }
    }
}

impl Say for Words {
    fn say(&self, key: &str) -> String {
        let id = format!("admin-{}", key.replace('.', "-"));
        let said = UiTexts::new(self.german).plain(&id);
        if said != id {
            return said;
        }
        match key.strip_prefix("wiki.") {
            Some(rest) => rest
                .rsplit('.')
                .next()
                .map_or_else(|| key.to_owned(), spelled),
            None => key.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frontends_words_come_in_both_languages_and_a_refusal_has_its_reason() {
        let (en, de) = (Words::new(false), Words::new(true));
        assert_eq!(en.say("action.save"), "Save");
        assert_eq!(de.say("action.save"), "Speichern");
        assert!(en.say("message.rejected").contains("path is taken"));
        assert!(de.say("message.rejected").contains("Pfad ist vergeben"));
        assert!(!en.say("message.unavailable").contains("unavailable"));
        assert_eq!(en.say("wiki.page.redirect-to"), "Redirect to");
        assert_eq!(en.say("something.unknown"), "something.unknown");
    }
}
