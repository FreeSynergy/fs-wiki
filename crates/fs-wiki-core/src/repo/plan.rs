//! The plan of a folder move: which texts move where.
//!
//! Pattern: Command producer (plan part) — made from reads only; nothing is written here.

use fs_objects::ObjectId;

use super::handle::WikiRepo;
use crate::error::Result;
use crate::texts::{Paging, TextEntry};

/// Texts are listed this many at a time while planning.
const BATCH: usize = 500;

/// One text to move.
pub(super) struct Step {
    pub id: ObjectId,
    pub old: String,
    pub new: String,
}

/// `path` is `folder` itself or lies below it.
pub(super) fn within(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|r| r.starts_with('/'))
}

pub(super) fn depth(path: &str) -> usize {
    path.split('/').count()
}

impl WikiRepo {
    /// The paths of all texts (the meaning of a link depends on which of its candidates exist).
    pub(super) async fn all_paths(&self) -> Result<std::collections::HashSet<String>> {
        let mut paths = std::collections::HashSet::new();
        loop {
            let page = self.list_texts("", Paging::new(BATCH, paths.len())).await?;
            if page.is_empty() {
                return Ok(paths);
            }
            paths.extend(page.into_iter().map(|t| t.path));
        }
    }

    /// Which texts move where; shallow ones first, so a place vacated by one is free for a
    /// deeper one. Reads only ids and paths into memory, a batch of texts at a time.
    pub(super) async fn plan(&self, old: &str, new: &str) -> Result<Vec<Step>> {
        let mut steps = vec![];
        let mut found = self.text_at(old).await?.into_iter().collect::<Vec<_>>();
        let mut offset = 0;
        loop {
            steps.extend(found.drain(..).map(|t| Step {
                id: t.id,
                new: format!("{new}{}", &t.path[old.len()..]),
                old: t.path,
            }));
            let page = Paging::new(BATCH, offset);
            found = self.list_texts(&format!("{old}/"), page).await?;
            if found.is_empty() {
                break;
            }
            offset += found.len();
        }
        steps.sort_by(|a, b| (depth(&a.old), &a.old).cmp(&(depth(&b.old), &b.old)));
        Ok(steps)
    }

    /// Whether a text is at `folder` or below it that `allowed` does not accept.
    pub(super) async fn occupied(
        &self,
        folder: &str,
        allowed: &(dyn Fn(&TextEntry) -> bool + Sync),
    ) -> Result<bool> {
        if self.text_at(folder).await?.is_some_and(|t| !allowed(&t)) {
            return Ok(true);
        }
        let mut offset = 0;
        loop {
            let page = Paging::new(BATCH, offset);
            let below = self.list_texts(&format!("{folder}/"), page).await?;
            if below.is_empty() {
                return Ok(false);
            }
            if below.iter().any(|t| !allowed(t)) {
                return Ok(true);
            }
            offset += below.len();
        }
    }

    /// The text at `folder` and the texts below it, as they are.
    pub(super) async fn entries_below(&self, folder: &str) -> Result<Vec<TextEntry>> {
        let mut all: Vec<TextEntry> = self.text_at(folder).await?.into_iter().collect();
        all.extend(
            self.list_texts(&format!("{folder}/"), Paging::default())
                .await?,
        );
        all.sort_by(|a, b| (depth(&a.path), &a.path).cmp(&(depth(&b.path), &b.path)));
        Ok(all)
    }
}
