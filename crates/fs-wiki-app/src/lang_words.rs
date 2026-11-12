//! The few sentences of the language bar and its notices, in German and English.
//!
//! Pattern: Dictionary — one table per language; a language without a table gets English. A sentence is whole, with named
//! places (`{lang}`), never built from pieces. (The reader's own labels are English; they follow with the words of
//! `fs-components`.)

/// The name of a language in that language.
pub fn language_name(code: &str) -> String {
    match code {
        "de" => "Deutsch".into(),
        "en" => "English".into(),
        other => other.to_owned(),
    }
}

/// The name of a language as a person speaking `ui` says it ("auf Englisch", "in German").
fn named_in(ui: &str, code: &str) -> String {
    match (ui, code) {
        ("de", "en") => "Englisch".into(),
        ("de", "de") => "Deutsch".into(),
        (_, "en") => "English".into(),
        (_, "de") => "German".into(),
        (_, other) => other.to_owned(),
    }
}

/// `Language:` / `Sprache:`.
pub fn language_label(ui: &str) -> &'static str {
    if ui == "de" { "Sprache" } else { "Language" }
}

/// The page is shown in another language than asked for because it has none in that one.
pub fn only_in(ui: &str, shown: &str) -> String {
    let lang = named_in(ui, shown);
    if ui == "de" {
        format!("Diese Seite gibt es nur auf {lang}.")
    } else {
        format!("This page is only available in {lang}.")
    }
}

/// The shown translation was made from an older state of its origin.
pub fn outdated(ui: &str, origin: &str) -> String {
    let lang = named_in(ui, origin);
    if ui == "de" {
        format!("Diese Übersetzung ist veraltet: das Original ({lang}) wurde danach geändert.")
    } else {
        format!(
            "This translation is outdated: the original ({lang}) was changed after it was translated."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentences_exist_in_both_languages() {
        assert_eq!(only_in("de", "en"), "Diese Seite gibt es nur auf Englisch.");
        assert_eq!(
            only_in("en", "de"),
            "This page is only available in German."
        );
        assert!(outdated("de", "en").contains("veraltet"));
        assert!(outdated("en", "de").contains("outdated"));
        assert_eq!(language_label("de"), "Sprache");
    }
}
