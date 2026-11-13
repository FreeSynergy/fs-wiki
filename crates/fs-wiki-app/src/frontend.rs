//! The frontend of the reading wiki (Classic, Confluence or Notion): chosen in the window, remembered, and the words in it.
//!
//! Pattern: Memento + Factory — the choice is one word in a small file in the data directory of the program, written
//! atomically (temporary file, rename); a missing or broken file means the default, never an error. The arguments and
//! the remembered choice go in, a frontend and the words of the interface language come out (German or English, as the
//! language bar's sentences in `lang_words.rs`).

use std::path::{Path, PathBuf};

use fs_components_workbenches::{Layout, WikiLabels};

use crate::args::Args;
use crate::store_path::data_dir;

/// How the reading wiki looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frontend {
    /// The plain reader: a list of pages and the page.
    Classic,
    /// Page tree on the left, head with language switch, outline on the right (`--frontend a`).
    Confluence,
    /// A calm sidebar and the page as blocks (`--frontend b`).
    Notion,
}

impl Frontend {
    /// All frontends, in the order of the menu.
    pub const ALL: [Frontend; 3] = [Frontend::Classic, Frontend::Confluence, Frontend::Notion];

    /// What a new installation shows: the page tree and the outline help most in a large documentation.
    pub const DEFAULT: Frontend = Frontend::Confluence;

    /// The word in the settings file and after `--frontend`.
    pub fn id(self) -> &'static str {
        match self {
            Frontend::Classic => "classic",
            Frontend::Confluence => "a",
            Frontend::Notion => "b",
        }
    }

    /// The frontend of a word (`a`, `b`, `classic`); none for anything else.
    pub fn parse(word: &str) -> Option<Frontend> {
        Frontend::ALL.into_iter().find(|f| f.id() == word.trim())
    }

    /// The layout of `fs-components` that draws it.
    pub fn layout(self) -> Layout {
        match self {
            Frontend::Classic => Layout::Classic,
            Frontend::Confluence => Layout::Confluence,
            Frontend::Notion => Layout::Notion,
        }
    }

    /// Whether the frontend draws the language switch itself (the page text then has no bar on top).
    pub fn has_own_head(self) -> bool {
        self != Frontend::Classic
    }
}

/// The settings file below the data directory `data`.
pub fn settings_file(data: &Path) -> PathBuf {
    data.join("freesynergy").join("fs-wiki").join("frontend")
}

/// The file of versions before 0.0.9, which called the frontends faces; read once, never deleted.
fn old_settings_file(file: &Path) -> PathBuf {
    file.with_file_name("face")
}

/// The remembered frontend; none when nothing is remembered or the file is broken. When there is no `frontend` file yet
/// but the old `face` file of an earlier version holds a valid word, that word is taken over: written as `frontend`
/// (atomically; the old file stays) and returned.
pub fn load(file: &Path) -> Option<Frontend> {
    match std::fs::read_to_string(file) {
        Ok(text) => Frontend::parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let old = Frontend::parse(&std::fs::read_to_string(old_settings_file(file)).ok()?)?;
            // A failed take-over is not an error: the old file is read again next time.
            let _ = save(file, old);
            Some(old)
        }
        Err(_) => None,
    }
}

/// Remember `frontend` in `file` (written under a temporary name, then renamed over it).
///
/// # Errors
/// The text of the failure; the window then goes on with the frontend it has and tells the person.
pub fn save(file: &Path, frontend: Frontend) -> Result<(), String> {
    let fail = |e: std::io::Error| e.to_string();
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(fail)?;
    }
    let temp = file.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, format!("{}\n", frontend.id())).map_err(fail)?;
    std::fs::rename(&temp, file).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        e.to_string()
    })
}

/// The frontend to start with: `--frontend` when given (not remembered), else the remembered one, else the default.
pub(crate) fn pick(over: Option<&str>, remembered: Option<Frontend>) -> Frontend {
    over.and_then(Frontend::parse)
        .or(remembered)
        .unwrap_or(Frontend::DEFAULT)
}

/// The settings file of the frontend, when there is a data directory.
pub(crate) fn settings() -> Option<PathBuf> {
    data_dir().map(|d| settings_file(&d))
}

/// The frontend of a window: the override, else what the settings file holds.
pub(crate) fn start_frontend(args: &Args, settings: Option<&PathBuf>) -> Frontend {
    pick(args.frontend.as_deref(), settings.and_then(|f| load(f)))
}

/// The frontend of a picture: only the override, else the plain reader (no setting is read, a picture is repeatable).
pub(crate) fn picture_frontend(args: &Args) -> Frontend {
    args.frontend
        .as_deref()
        .and_then(Frontend::parse)
        .unwrap_or(Frontend::Classic)
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

/// The words of the wiki layouts, in the interface language (`texts/`, `ui_texts.rs`).
pub(crate) fn words(german: bool) -> WikiLabels {
    crate::ui_texts::UiTexts::new(german).wiki_labels()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_override_beats_the_remembered_frontend_which_beats_the_default() {
        assert_eq!(pick(Some("b"), Some(Frontend::Classic)), Frontend::Notion);
        assert_eq!(pick(Some("classic"), None), Frontend::Classic);
        assert_eq!(pick(None, Some(Frontend::Notion)), Frontend::Notion);
        assert_eq!(pick(None, None), Frontend::Confluence);
    }
}
