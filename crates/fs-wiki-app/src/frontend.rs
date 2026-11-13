//! Which wiki frame (`--frontend a|b`) and the words in it.
//!
//! Pattern: Factory — the arguments in, a layout and the words of the interface language out (German or English, as the
//! language bar's sentences in `lang_words.rs`).

use fs_components_workbenches::{Layout, WikiLabels};

use crate::args::Args;

/// The layout asked for and its words; without `--frontend` the plain reader.
pub(crate) fn choice(args: &Args) -> (Layout, WikiLabels) {
    let layout = match args.frontend.as_deref() {
        Some("a") => Layout::Confluence,
        Some("b") => Layout::Notion,
        _ => Layout::Classic,
    };
    let german = args
        .lang
        .as_deref()
        .map_or_else(env_is_german, |l| l == "de");
    (
        layout,
        if german {
            german_words()
        } else {
            WikiLabels::default()
        },
    )
}

fn env_is_german() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .is_some_and(|v| v.starts_with("de"))
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
