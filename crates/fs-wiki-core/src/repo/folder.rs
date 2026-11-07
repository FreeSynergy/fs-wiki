//! Moving a folder: the text at a path and everything below it, with the links.
//!
//! Pattern: Command producer + plan/execute — first a plan is made from reads only (which texts
//! move where, which clash), then it is executed in two phases. Nothing is written while a plan
//! is refused.
//!
//! Order of writes, chosen so that repeating an interrupted call finishes it (`fs-objects` 0.2 has
//! no transaction over several objects):
//! 1. every text outside the folder that links into it is rewritten (found by the `link` index,
//!    not by reading all texts);
//! 2. the texts of the folder move one by one, shallow ones first; each is written once, with its
//!    new path and its links already rewritten.
//!
//! A repeated call plans again from what is stored: rewritten texts no longer match phase 1 and
//! moved texts no longer lie below the old folder, so only the rest is done. This needs the new
//! places to lie outside the old folder; a target that would put a moved text inside the old
//! folder (moving up into an ancestor where a name repeats) is refused for that reason.

use std::collections::HashSet;

use fs_objects::ObjectId;
use fs_text::{rewrite_links_with, Document, MovePrefix};

use super::handle::WikiRepo;
use super::read::all_texts;
use crate::error::{Error, Result};
use crate::path::TextPath;
use crate::texts::{Paging, TextEntry};

/// Texts are listed this many at a time while planning.
const BATCH: usize = 500;

/// One text to move.
struct Step {
    id: ObjectId,
    old: String,
    new: String,
}

/// `path` is `folder` itself or lies below it.
fn within(path: &str, folder: &str) -> bool {
    path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|r| r.starts_with('/'))
}

fn depth(path: &str) -> usize {
    path.split('/').count()
}

impl WikiRepo {
    pub(super) async fn move_folder_locked(
        &self,
        from: &TextPath,
        to: &TextPath,
    ) -> Result<Vec<TextEntry>> {
        let (old, new) = (from.as_str(), to.as_str());
        if old == new {
            return self.entries_below(old).await;
        }
        let invalid = |reason| Error::InvalidMove {
            from: old.into(),
            to: new.into(),
            reason,
        };
        if within(new, old) {
            return Err(invalid("the target lies inside the folder itself"));
        }
        let steps = self.plan(old, new).await?;
        if steps.is_empty() {
            return Err(Error::NoSuchFolder(old.into()));
        }
        if steps.iter().any(|s| within(&s.new, old)) {
            return Err(invalid(
                "a moved text would land inside the old folder, which an interrupted move could not tell apart",
            ));
        }
        for step in &steps {
            if self.text_at(&step.new).await?.is_some() {
                return Err(Error::PathTaken(step.new.clone()));
            }
        }

        let rewriter = MovePrefix::new(old, new);
        self.rewrite_outside_links(old, &rewriter, &steps).await?;
        for step in &steps {
            let text = self.text(step.id).await?.ok_or(Error::NotFound(step.id))?;
            let mut docs: [Document; 1] = [text.document];
            rewrite_links_with(&mut docs, &rewriter);
            let [doc] = docs;
            self.store_document(&text.envelope, &step.new, &doc).await?;
        }
        let mut moved = Vec::with_capacity(steps.len());
        for step in &steps {
            moved.push(self.entry(step.id).await?);
        }
        Ok(moved)
    }

    /// Which texts move where; shallow ones first, so a place vacated by one is free for a
    /// deeper one. Reads only ids and paths into memory, a batch of texts at a time.
    async fn plan(&self, old: &str, new: &str) -> Result<Vec<Step>> {
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

    /// Phase 1: texts that are not part of the move but link into the folder.
    async fn rewrite_outside_links(
        &self,
        old: &str,
        rewriter: &MovePrefix,
        steps: &[Step],
    ) -> Result<()> {
        let moving: HashSet<ObjectId> = steps.iter().map(|s| s.id).collect();
        let exact = all_texts(Paging::default()).with_index("link", old);
        let below = all_texts(Paging::default()).index_between(
            "link",
            format!("{old}/"),
            format!("{old}/\u{10FFFF}"),
        );
        let mut done: HashSet<ObjectId> = HashSet::new();
        for query in [exact, below] {
            for other in self.reads.query(&query).await? {
                let id = other.envelope.id;
                if moving.contains(&id) || !done.insert(id) {
                    continue;
                }
                let entry = super::record::entry_of(other)?;
                let mut docs = [entry.document];
                if !rewrite_links_with(&mut docs, rewriter).is_empty() {
                    let [doc] = docs;
                    self.store_document(&entry.envelope, &entry.path, &doc)
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// The text at `folder` and the texts below it, as they are.
    async fn entries_below(&self, folder: &str) -> Result<Vec<TextEntry>> {
        let mut all: Vec<TextEntry> = self.text_at(folder).await?.into_iter().collect();
        all.extend(
            self.list_texts(&format!("{folder}/"), Paging::default())
                .await?,
        );
        all.sort_by(|a, b| (depth(&a.path), &a.path).cmp(&(depth(&b.path), &b.path)));
        Ok(all)
    }
}
