//! The face of the reading wiki: Classic, Confluence or Notion, chosen in the window and remembered.
//!
//! Pattern: Memento — the choice is one word in a small file in the data directory of the program, written atomically
//! (temporary file, rename); a missing or broken file means the default, never an error.

use std::path::{Path, PathBuf};

use fs_components_workbenches::Layout;

/// How the reading wiki looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// The plain reader: a list of pages and the page.
    Classic,
    /// Page tree on the left, head with language switch, outline on the right (`--frontend a`).
    Confluence,
    /// A calm sidebar and the page as blocks (`--frontend b`).
    Notion,
}

impl Face {
    /// All faces, in the order of the menu.
    pub const ALL: [Face; 3] = [Face::Classic, Face::Confluence, Face::Notion];

    /// What a new installation shows: the page tree and the outline help most in a large documentation.
    pub const DEFAULT: Face = Face::Confluence;

    /// The word in the settings file and after `--frontend`.
    pub fn id(self) -> &'static str {
        match self {
            Face::Classic => "classic",
            Face::Confluence => "a",
            Face::Notion => "b",
        }
    }

    /// The face of a word (`a`, `b`, `classic`); none for anything else.
    pub fn parse(word: &str) -> Option<Face> {
        Face::ALL.into_iter().find(|f| f.id() == word.trim())
    }

    /// The layout of `fs-components` that draws it.
    pub fn layout(self) -> Layout {
        match self {
            Face::Classic => Layout::Classic,
            Face::Confluence => Layout::Confluence,
            Face::Notion => Layout::Notion,
        }
    }

    /// Whether the frontend draws the language switch itself (the page text then has no bar on top).
    pub fn has_own_head(self) -> bool {
        self != Face::Classic
    }
}

/// The settings file below the data directory `data`.
pub fn settings_file(data: &Path) -> PathBuf {
    data.join("freesynergy").join("fs-wiki").join("face")
}

/// The remembered face; none when nothing is remembered or the file is broken.
pub fn load(file: &Path) -> Option<Face> {
    Face::parse(&std::fs::read_to_string(file).ok()?)
}

/// Remember `face` in `file` (written under a temporary name, then renamed over it).
///
/// # Errors
/// The text of the failure; the window then goes on with the face it has and tells the person.
pub fn save(file: &Path, face: Face) -> Result<(), String> {
    let fail = |e: std::io::Error| e.to_string();
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(fail)?;
    }
    let temp = file.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, format!("{}\n", face.id())).map_err(fail)?;
    std::fs::rename(&temp, file).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        e.to_string()
    })
}
