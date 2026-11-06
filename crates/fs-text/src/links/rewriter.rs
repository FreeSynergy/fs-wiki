//! The link rewriting service interface and the "page / folder moved" strategy.
//!
//! Pattern: Strategy — [`LinkRewriter`] maps a link path to its new path (or leaves it);
//! [`MovePrefix`] is the rename/move strategy, others (merge, slugify) can follow.

/// Maps a link path to a new one.
pub trait LinkRewriter {
    /// New path for `path`, or `None` if the link is not affected.
    fn rewrite(&self, path: &str) -> Option<String>;
}

/// Page or folder move: `old` and `old/…` become `new` and `new/…`; `#fragment` is kept.
#[derive(Clone, Debug)]
pub struct MovePrefix {
    /// Old page or folder path.
    pub old: String,
    /// New page or folder path.
    pub new: String,
}

impl MovePrefix {
    /// Move `old` to `new`.
    #[must_use]
    pub fn new(old: &str, new: &str) -> Self {
        Self {
            old: old.to_string(),
            new: new.to_string(),
        }
    }
}

impl LinkRewriter for MovePrefix {
    fn rewrite(&self, path: &str) -> Option<String> {
        moved_path(path, &self.old, &self.new)
    }
}

/// `path` with the moved prefix replaced, or `None` if the link is not affected.
/// Matches the page itself, `old/…` (folder move) and keeps any `#fragment`.
#[must_use]
pub fn moved_path(path: &str, old: &str, new: &str) -> Option<String> {
    let (page, frag) = match path.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (path, None),
    };
    let moved = if page == old {
        new.to_string()
    } else {
        let rest = page.strip_prefix(old).and_then(|r| r.strip_prefix('/'))?;
        format!("{new}/{rest}")
    };
    Some(match frag {
        Some(f) => format!("{moved}#{f}"),
        None => moved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_pages_folders_and_keeps_fragments() {
        let r = MovePrefix::new("a", "b");
        assert_eq!(r.rewrite("a"), Some("b".into()));
        assert_eq!(r.rewrite("a/c#h"), Some("b/c#h".into()));
        assert_eq!(r.rewrite("ab"), None);
        assert_eq!(r.rewrite("https://x/a"), None);
    }
}
