//! Making a new page: a title in, a free path and a text with that title as its first heading out.
//!
//! Pattern: Factory + Adapter — `PageMaker` is what the window needs (so a test can put a small source in its place);
//! `WikiSource` implements it over `Texts::create`. The path is derived from the title and the parent page, never typed
//! by the person: `<folder of the parent>/<title as a word>.md`. A taken path is not an error of the person: the title
//! gets a number (`New page 2`), because a new page is made at once and named in the editor (the Confluence way: an
//! untitled draft, no dialog in between).

use std::sync::Arc;

use fs_wiki_core::Error;

use super::wiki::WikiSource;

/// Most titles tried before it is said that the path is taken.
const TRIES: usize = 99;
/// Longest word of a path made from a title (characters).
const WORD_MAX: usize = 80;

/// Why a page was not made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateError {
    /// The window is read only.
    ReadOnly,
    /// The title has no letter or digit to make a path from.
    EmptyTitle,
    /// The path (and all numbered ones) is taken.
    Taken(String),
    /// The path is not allowed; the core's reason.
    Invalid(String),
    /// The server or the store failed; its words.
    Failed(String),
}

/// What is called after an edit was saved.
pub type SavedHook = Arc<dyn Fn() + Send + Sync>;

/// What the window needs to make pages.
pub trait PageMaker: Send + Sync {
    /// Whether pages can be made (not in a read only window).
    fn can_create(&self) -> bool;

    /// Make a page called `title` (numbered when taken) below `parent` (a page; `None` = the top level) and return its
    /// path. The text is the title as a first-level heading.
    ///
    /// # Errors
    /// [`CreateError`] with the reason; nothing is made then.
    fn create_page(&self, parent: Option<&str>, title: &str) -> Result<String, CreateError>;

    /// Call `hook` after every saved edit, from the thread that saves (it must not wait for the reader). The default never
    /// calls it.
    fn on_saved(&self, _hook: SavedHook) {}
}

impl WikiSource {
    /// Tell the window that an edit was saved.
    pub(super) fn saved(&self) {
        let hook = self
            .on_saved
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(hook) = hook {
            hook();
        }
    }
}

/// `title` as one word of a path: letters and digits in lower case, other characters between them as one `-`.
pub fn path_word(title: &str) -> Option<String> {
    let mut word = String::new();
    for c in title.trim().chars() {
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else if (c.is_whitespace() || c == '-' || c == '_') && !word.ends_with('-') {
            word.push('-');
        }
    }
    let word: String = word.trim_matches('-').chars().take(WORD_MAX).collect();
    let word = word.trim_matches('-').to_owned();
    (!word.is_empty()).then_some(word)
}

/// The folder the children of the page `parent` live in: `a/b.md` -> `a/b`, `a/index.md` -> `a`, none -> the top level.
pub fn folder_of(parent: Option<&str>) -> String {
    let Some(parent) = parent else {
        return String::new();
    };
    let stem = parent.strip_suffix(".md").unwrap_or(parent);
    match stem.rsplit_once('/') {
        Some((folder, "index")) => folder.to_owned(),
        None if stem == "index" => String::new(),
        _ => stem.to_owned(),
    }
}

/// The path of a page with this `title` below `parent`.
pub fn page_path(parent: Option<&str>, title: &str) -> Option<String> {
    let word = path_word(title)?;
    let folder = folder_of(parent);
    Some(if folder.is_empty() {
        format!("{word}.md")
    } else {
        format!("{folder}/{word}.md")
    })
}

impl PageMaker for WikiSource {
    fn can_create(&self) -> bool {
        !self.read_only
    }

    fn on_saved(&self, hook: SavedHook) {
        *self
            .on_saved
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(hook);
    }

    fn create_page(&self, parent: Option<&str>, title: &str) -> Result<String, CreateError> {
        if self.read_only {
            return Err(CreateError::ReadOnly);
        }
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut last = String::new();
        for n in 1..=TRIES {
            let numbered = if n == 1 {
                title.clone()
            } else {
                format!("{title} {n}")
            };
            let path = page_path(parent, &numbered).ok_or(CreateError::EmptyTitle)?;
            last.clone_from(&path);
            if self.exists(&path) {
                continue;
            }
            match self
                .runtime
                .block_on(self.texts.create(&path, &format!("# {numbered}\n")))
            {
                Ok(made) => return Ok(made.path),
                // Someone made it between the look and the write: the next number.
                Err(Error::PathTaken(_)) => {}
                Err(Error::InvalidPath { reason, .. }) => {
                    return Err(CreateError::Invalid(reason.to_string()));
                }
                Err(e) => return Err(CreateError::Failed(e.to_string())),
            }
        }
        Err(CreateError::Taken(last))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_becomes_a_word_of_a_path() {
        assert_eq!(path_word("New page").as_deref(), Some("new-page"));
        assert_eq!(
            path_word("  Neue  Seite 2 ").as_deref(),
            Some("neue-seite-2")
        );
        assert_eq!(path_word("Größe / Maß: #1").as_deref(), Some("größe-maß-1"));
        assert_eq!(path_word("../..").as_deref(), None);
        assert_eq!(path_word("   ").as_deref(), None);
    }

    #[test]
    fn a_child_lives_in_the_folder_of_its_parent() {
        assert_eq!(page_path(None, "A").as_deref(), Some("a.md"));
        assert_eq!(page_path(Some("x.md"), "A").as_deref(), Some("x/a.md"));
        assert_eq!(page_path(Some("x/y.md"), "A").as_deref(), Some("x/y/a.md"));
        assert_eq!(
            page_path(Some("x/index.md"), "A").as_deref(),
            Some("x/a.md")
        );
        assert_eq!(page_path(Some("index.md"), "A").as_deref(), Some("a.md"));
    }
}
