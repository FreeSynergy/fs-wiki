//! Which face the reading wiki starts with and the words in it.
//!
//! Pattern: Factory — the arguments and the remembered choice in, a face and the words of the interface language out
//! (German or English, as the language bar's sentences in `lang_words.rs`).

use std::path::PathBuf;

use fs_components_workbenches::WikiLabels;

use crate::args::Args;
use crate::face::{self, Face};
use crate::store_path::data_dir;

/// The face to start with: `--frontend` when given (not remembered), else the remembered one, else the default.
pub(crate) fn pick(over: Option<&str>, remembered: Option<Face>) -> Face {
    over.and_then(Face::parse)
        .or(remembered)
        .unwrap_or(Face::DEFAULT)
}

/// The settings file of the face, when there is a data directory.
pub(crate) fn settings() -> Option<PathBuf> {
    data_dir().map(|d| face::settings_file(&d))
}

/// The face of a window: the override, else what the settings file holds.
pub(crate) fn start_face(args: &Args, settings: Option<&PathBuf>) -> Face {
    pick(
        args.frontend.as_deref(),
        settings.and_then(|f| face::load(f)),
    )
}

/// The face of a picture: only the override, else the plain reader (no setting is read, a picture is repeatable).
pub(crate) fn picture_face(args: &Args) -> Face {
    args.frontend
        .as_deref()
        .and_then(Face::parse)
        .unwrap_or(Face::Classic)
}

/// Whether the interface is German: `--lang`, else the environment.
pub(crate) fn is_german(args: &Args) -> bool {
    args.lang
        .as_deref()
        .map_or_else(env_is_german, |l| l == "de")
}

fn env_is_german() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .is_some_and(|v| v.starts_with("de"))
}

/// The words of the wiki layouts.
pub(crate) fn words(german: bool) -> WikiLabels {
    if german {
        german_words()
    } else {
        WikiLabels::default()
    }
}

fn german_words() -> WikiLabels {
    let mut w = WikiLabels::default();
    let s = |t: &str| t.to_string();
    w.program = s("FreeSynergy Wiki");
    w.pages = s("Seiten");
    w.filter = s("Nach Titel suchen");
    w.outline = s("Gliederung");
    w.no_headings = s("Diese Seite hat keine Überschriften.");
    w.history = s("Verlauf");
    w.no_history = s("Keine früheren Fassungen bekannt.");
    w.state_current = s("Aktuell");
    w.state_outdated = s("Veraltet");
    w.state_fallback = s("Andere Sprache");
    w.language = s("Sprache");
    w.edit = s("Bearbeiten");
    w.share = s("Teilen");
    w.block_hint = s("Schreiben, oder / für Befehle");
    w.page_count = s("{} Seiten");
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_override_beats_the_remembered_face_which_beats_the_default() {
        assert_eq!(pick(Some("b"), Some(Face::Classic)), Face::Notion);
        assert_eq!(pick(Some("classic"), None), Face::Classic);
        assert_eq!(pick(None, Some(Face::Notion)), Face::Notion);
        assert_eq!(pick(None, None), Face::Confluence);
    }
}
