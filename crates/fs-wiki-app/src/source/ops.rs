//! What the window does to a page: make it under a name, rename it, move it to the trash.
//!
//! Pattern: Command over `Texts` — every operation is a few calls of the core in an order that can be undone: a rename moves
//! the page first (`move_to` keeps the id, so the history stays, and the links to it follow), then its subpages
//! (`move_folder`; if that fails the page is moved back), and only then writes the new title into the text; a delete is
//! `trash` (the page and its history move out of sight and can be restored), never `delete`. The names are checked here, so a
//! window cannot ask for a path the core would refuse in a way the person cannot read.

use fs_wiki_core::{Error, MARKDOWN, Paging, TextFormat};

use super::create::{CreateError, folder_of, page_path, path_word};
use super::wiki::WikiSource;

/// The name as it is taken: blanks collapsed. Refused: nothing, a slash, a backslash or a control character, a name without a
/// letter or digit.
///
/// # Errors
/// [`CreateError::EmptyTitle`], [`CreateError::BadChars`] or [`CreateError::NoLetter`].
pub fn check_name(title: &str) -> Result<String, CreateError> {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return Err(CreateError::EmptyTitle);
    }
    if title.chars().any(|c| c == '/' || c == '\\' || c.is_control()) {
        return Err(CreateError::BadChars);
    }
    path_word(&title).ok_or(CreateError::NoLetter)?;
    Ok(title)
}

/// The path of the page `path` under the name `title`: the same folder, the word of the title.
fn sibling(path: &str, title: &str) -> Option<String> {
    let word = path_word(title)?;
    Some(match path.rsplit_once('/') {
        Some((folder, _)) => format!("{folder}/{word}.md"),
        None => format!("{word}.md"),
    })
}

/// The first title line of `markdown` with the title `new`: the `title:` of the front matter, and the first line of the text
/// when it is a level-1 heading that says `old` (or when there is no front matter title); a text without either gets a heading.
pub fn retitle(markdown: &str, old: &str, new: &str) -> String {
    let (front, body) = match markdown.strip_prefix("---\n").and_then(|r| r.split_once("\n---\n")) {
        Some((front, body)) => (Some(front), body),
        None => (None, markdown),
    };
    let has_front_title = front.is_some_and(|f| f.lines().any(|l| l.starts_with("title:")));
    let mut out = String::new();
    if let Some(front) = front {
        out.push_str("---\n");
        for line in front.lines() {
            if line.starts_with("title:") {
                out.push_str(&format!("title: \"{}\"\n", new.replace('\\', "\\\\").replace('"', "\\\"")));
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str("---\n");
    }
    let lead_end = body.find(|c: char| !c.is_whitespace()).unwrap_or(body.len());
    let (blank, rest) = body.split_at(lead_end);
    let first_line = rest.lines().next().unwrap_or("");
    let is_old_h1 = first_line
        .strip_prefix("# ")
        .is_some_and(|h| h.trim() == old.trim() || !has_front_title);
    if is_old_h1 {
        out.push_str(blank);
        out.push_str(&format!("# {new}"));
        out.push_str(&rest[first_line.len()..]);
    } else if has_front_title {
        out.push_str(body);
    } else {
        out.push_str(&format!("# {new}\n\n{body}"));
    }
    out
}

impl WikiSource {
    fn runs<T>(&self, f: impl std::future::Future<Output = fs_wiki_core::Result<T>>) -> Result<T, CreateError> {
        self.runtime.block_on(f).map_err(|e| match e {
            Error::PathTaken(p) => CreateError::Taken(p),
            Error::TargetNotEmpty { to, .. } => CreateError::Taken(to),
            Error::InvalidPath { reason, .. } | Error::InvalidMove { reason, .. } => {
                CreateError::Invalid(reason.to_string())
            }
            other => CreateError::Failed(other.to_string()),
        })
    }

    fn may_change(&self, path: &str) -> Result<(), CreateError> {
        if self.read_only {
            return Err(CreateError::ReadOnly);
        }
        if self.pages.is_some() {
            return Err(CreateError::Variants);
        }
        if !self.exists(path) {
            return Err(CreateError::NotFound(path.to_owned()));
        }
        Ok(())
    }

    /// A page called exactly `title` below `parent`; a taken path is an error.
    pub(super) fn create_named(&self, parent: Option<&str>, title: &str) -> Result<String, CreateError> {
        if self.read_only {
            return Err(CreateError::ReadOnly);
        }
        let title = check_name(title)?;
        let path = page_path(parent, &title).ok_or(CreateError::NoLetter)?;
        if self.exists(&path) {
            return Err(CreateError::Taken(path));
        }
        Ok(self.runs(self.texts.create(&path, &format!("# {title}\n")))?.path)
    }

    /// The pages below `path` (its folder), not the page itself.
    fn below(&self, path: &str) -> Vec<fs_wiki_core::TextEntry> {
        let folder = folder_of(Some(path));
        if folder.is_empty() || path.ends_with("index.md") {
            return Vec::new();
        }
        let prefix = format!("{folder}/");
        self.runtime
            .block_on(self.texts.list(&prefix, Paging::default()))
            .unwrap_or_default()
    }

    pub(super) fn subpages(&self, path: &str) -> usize {
        self.below(path).len()
    }

    /// Rename: see the module's comment.
    pub(super) fn rename(&self, path: &str, title: &str) -> Result<String, CreateError> {
        self.may_change(path)?;
        if path.ends_with("index.md") {
            return Err(CreateError::IndexPage);
        }
        let title = check_name(title)?;
        let new_path = sibling(path, &title).ok_or(CreateError::NoLetter)?;
        let entry = self.entry(path).map_err(|e| CreateError::Failed(e.to_string()))?;
        let old_title = entry.envelope.name.clone();
        let (old_folder, new_folder) = (folder_of(Some(path)), folder_of(Some(&new_path)));
        let has_children = !self.below(path).is_empty();
        if new_path != path {
            if self.exists(&new_path) {
                return Err(CreateError::Taken(new_path));
            }
            self.runs(self.texts.move_to(entry.id, &new_path))?;
            if has_children
                && let Err(e) = self.runs(self.texts.move_folder(&old_folder, &new_folder))
            {
                // The page goes back: the rename is all or nothing.
                let _ = self.runs(self.texts.move_to(entry.id, path));
                return Err(e);
            }
        }
        self.write_title(&new_path, &old_title, &title)
            .map_err(CreateError::Partial)?;
        self.saved();
        Ok(new_path)
    }

    fn write_title(&self, path: &str, old: &str, new: &str) -> Result<(), String> {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let entry = self.entry(path).map_err(|e| fail(&e))?;
        let text = self
            .runtime
            .block_on(self.texts.export_text(entry.id, MARKDOWN))
            .map_err(|e| fail(&e))?;
        let changed = retitle(&text, old, new);
        if changed == text {
            return Ok(());
        }
        let mut document = entry.document.clone();
        TextFormat::Markdown.import_external(&mut document, &changed);
        self.runtime
            .block_on(self.texts.edit_if(entry.id, entry.revision, document))
            .map(|_| ())
            .map_err(|e| fail(&e))
    }

    /// Trash the page and the pages below it, the subpages first; how many.
    pub(super) fn trash_with_subpages(&self, path: &str) -> Result<usize, CreateError> {
        self.may_change(path)?;
        let entry = self.entry(path).map_err(|e| CreateError::Failed(e.to_string()))?;
        let mut count = 0;
        for child in self.below(path) {
            self.runs(self.texts.trash(child.id))?;
            count += 1;
        }
        self.runs(self.texts.trash(entry.id))?;
        self.saved();
        Ok(count + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_checked_before_anything_is_asked() {
        assert_eq!(check_name("  Hand   book ").as_deref(), Ok("Hand book"));
        assert_eq!(check_name("   "), Err(CreateError::EmptyTitle));
        assert_eq!(check_name("a/b"), Err(CreateError::BadChars));
        assert_eq!(check_name("a\\b"), Err(CreateError::BadChars));
        assert_eq!(check_name("a\u{7}b"), Err(CreateError::BadChars));
        assert_eq!(check_name("?!"), Err(CreateError::NoLetter));
    }

    #[test]
    fn a_page_stays_in_its_folder_when_renamed() {
        assert_eq!(sibling("a.md", "New Name").as_deref(), Some("new-name.md"));
        assert_eq!(sibling("x/y/a.md", "B").as_deref(), Some("x/y/b.md"));
    }

    #[test]
    fn the_title_of_the_text_follows_the_name() {
        assert_eq!(retitle("# Old\n\ntext\n", "Old", "New"), "# New\n\ntext\n");
        assert_eq!(retitle("text\n", "Old", "New"), "# New\n\ntext\n");
        assert_eq!(retitle("\n# Old\nx\n", "Old", "New"), "\n# New\nx\n");
        // With a title in the front matter the heading changes only when it said the old title.
        assert_eq!(
            retitle("---\ntitle: Old\n---\n# Old\nx\n", "Old", "N \"q\""),
            "---\ntitle: \"N \\\"q\\\"\"\n---\n# N \"q\"\nx\n"
        );
        assert_eq!(
            retitle("---\ntitle: Old\n---\n# Else\nx\n", "Old", "New"),
            "---\ntitle: \"New\"\n---\n# Else\nx\n"
        );
    }
}
