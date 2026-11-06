//! What a move does to the meaning of a link.
//!
//! Pattern: Strategy data — a page (`folder == false`) or a folder (`true`) moves from `old` to
//! `new`. A link means the first existing text among its candidates (`x`, `x.md`, `x/index.md`,
//! `x/README.md`, in the order a reader tries them); only if that text moves does the link
//! change, and its new form must lead to the moved text again, else it is written in full.

use std::collections::HashSet;

use super::target::{candidates_of, Written};

/// A page or a folder that moves, and the paths of all texts before the move.
pub(super) struct Shift<'a> {
    pub old: &'a str,
    pub new: &'a str,
    pub folder: bool,
    pub paths: &'a HashSet<String>,
}

impl Shift<'_> {
    /// Where the text at `path` goes; `None` if it stays.
    fn moved(&self, path: &str) -> Option<String> {
        if path == self.old {
            return Some(self.new.to_string());
        }
        let rest = path.strip_prefix(self.old)?.strip_prefix('/')?;
        self.folder.then(|| format!("{}/{rest}", self.new))
    }

    /// Where the text now at `path` came from, if it was moved there.
    fn origin(&self, path: &str) -> Option<String> {
        if path == self.new {
            return Some(self.old.to_string());
        }
        let rest = path.strip_prefix(self.new)?.strip_prefix('/')?;
        self.folder.then(|| format!("{}/{rest}", self.old))
    }

    fn exists_after(&self, path: &str) -> bool {
        match self.origin(path) {
            Some(from) => self.paths.contains(&from),
            None => self.moved(path).is_none() && self.paths.contains(path),
        }
    }

    /// The text the link means: the first candidate that exists; a link to nothing means its
    /// path (so a dangling link into a moved folder still follows it).
    fn meaning(&self, w: &Written) -> String {
        w.candidates()
            .into_iter()
            .find(|c| self.paths.contains(c))
            .unwrap_or_else(|| w.target.clone())
    }

    /// The text the link means stays, but the move may have put a text in front of it in the
    /// reader's order (`z.md` before `z/index.md`): then the link is written in full.
    fn keep_meaning(&self, w: &Written, meant: &str) -> Option<(String, bool)> {
        let first = candidates_of(&w.target)
            .into_iter()
            .find(|c| self.exists_after(c));
        (self.paths.contains(meant) && first.is_some_and(|f| f != meant))
            .then(|| (meant.to_string(), false))
    }

    /// The new target path of the link `w`, and whether it is still a folder form (a trailing `/`
    /// may stay); `None` if the text it means does not move.
    pub(super) fn new_target(&self, w: &Written) -> Option<(String, bool)> {
        let meant = self.meaning(w);
        let Some(now) = self.moved(&meant) else {
            return self.keep_meaning(w, &meant);
        };
        let t = &w.target;
        let (target, folder_form) = if meant == *t {
            (now.clone(), true)
        } else if meant == format!("{t}.md") {
            (now.strip_suffix(".md").unwrap_or(&now).to_string(), false)
        } else {
            let named = ["/index.md", "/README.md"]
                .into_iter()
                .find(|n| meant == format!("{t}{n}"));
            match named
                .and_then(|n| now.strip_suffix(n))
                .filter(|p| !p.is_empty())
            {
                Some(parent) => (parent.to_string(), true),
                None => (now.clone(), false),
            }
        };
        let leads_there = candidates_of(&target)
            .into_iter()
            .find(|c| self.exists_after(c))
            .is_some_and(|c| c == now);
        Some(if leads_there || !self.paths.contains(&meant) {
            (target, folder_form)
        } else {
            (now, false)
        })
    }
}
