//! Moving a folder: the text at a path and everything below it, with the links.
//!
//! Pattern: Command producer + plan/execute — first a plan is made from reads only (which texts
//! move where, which clash), then it is executed in two phases. Nothing is written while a plan
//! is refused.
//!
//! Order of writes, chosen so that repeating an interrupted call finishes it (`fs-objects` 0.2 has
//! no transaction over several objects):
//! 0. the move is recorded (`move_log.rs`; "unfinished");
//! 1. every text outside the folder that links into it is rewritten (found by the `link` index,
//!    not by reading all texts);
//! 2. the texts of the folder move one by one, shallow ones first; each is written once, with its
//!    new path and its links already rewritten.
//!
//! Last, the record is marked done. A move never merges into texts that are already at the
//! target, and no other move starts while one is unfinished. A repeated call plans again from what is stored: rewritten texts no longer match phase 1 and
//! moved texts no longer lie below the old folder, so only the rest is done. This needs the new
//! places to lie outside the old folder; a target that would put a moved text inside the old
//! folder (moving up into an ancestor where a name repeats) is refused for that reason.

use std::collections::HashSet;

use fs_objects::ObjectId;

use super::handle::WikiRepo;
use super::move_log::MoveRecord;
use super::plan::{within, Step};
use super::read::all_texts;
use super::record::entry_of;
use super::relink::relinked;
use super::shift::Shift;
use crate::error::{Error, Result};
use crate::path::TextPath;
use crate::reason::Reason;
use crate::texts::{Paging, TextEntry};

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
            return Err(invalid(Reason::InsideItself));
        }
        let steps = self.plan(old, new).await?;
        let last = self.last_move().await?;
        let same = last.as_ref().filter(|r| r.is(old, new));
        if steps.is_empty() {
            // Nothing left to move: a repeat of a move that is complete, if the record says it
            // was this one and the texts are at the target.
            let moved = self.entries_below(new).await?;
            if let Some(r) = same.filter(|_| !moved.is_empty()) {
                self.record_move(&MoveRecord {
                    done: true,
                    ..r.clone()
                })
                .await?;
                return Ok(moved);
            }
            return Err(Error::NoSuchFolder(old.into()));
        }
        if steps.iter().any(|s| within(&s.new, old)) {
            return Err(invalid(Reason::InsideOld));
        }
        if let Some(r) = last.as_ref().filter(|r| !r.done && same.is_none()) {
            return Err(Error::MoveUnfinished {
                from: r.from.clone(),
                to: r.to.clone(),
            });
        }
        // The target may hold texts of this very move (finishing it) and nothing else.
        let mut ids: Vec<ObjectId> = same.map(|r| r.ids.clone()).unwrap_or_default();
        let ours = |t: &TextEntry| within(&t.path, old) || ids.contains(&t.id);
        if self.occupied(new, &ours).await? {
            return Err(Error::TargetNotEmpty {
                from: old.into(),
                to: new.into(),
            });
        }
        ids.extend(
            steps
                .iter()
                .map(|s| s.id)
                .filter(|id| !ids.contains(id))
                .collect::<Vec<_>>(),
        );
        for step in &steps {
            if self.text_at(&step.new).await?.is_some() {
                return Err(Error::PathTaken(step.new.clone()));
            }
        }

        let record = MoveRecord {
            from: old.into(),
            to: new.into(),
            done: false,
            ids,
        };
        self.record_move(&record).await?;
        let paths = self.all_paths().await?;
        let shift = Shift {
            old,
            new,
            folder: true,
            paths: &paths,
        };
        self.rewrite_outside_links(&shift, &steps).await?;
        for step in &steps {
            let text = self.text(step.id).await?.ok_or(Error::NotFound(step.id))?;
            let doc = relinked(&text.document, &step.old, &step.new, &shift)
                .unwrap_or_else(|| text.document.clone());
            self.store_document(&text, &step.new, &doc).await?;
        }
        self.record_move(&MoveRecord {
            done: true,
            ..record
        })
        .await?;
        let mut moved = Vec::with_capacity(steps.len());
        for step in &steps {
            moved.push(self.entry(step.id).await?);
        }
        Ok(moved)
    }

    /// Phase 1: texts that are not part of the move but link into the folder.
    async fn rewrite_outside_links(&self, shift: &Shift<'_>, steps: &[Step]) -> Result<()> {
        let old = shift.old;
        let moving: HashSet<ObjectId> = steps.iter().map(|s| s.id).collect();
        let exact = all_texts(Paging::default()).with_index("link", old);
        let below = all_texts(Paging::default()).index_between(
            "link",
            format!("{old}/"),
            format!("{old}/\u{10FFFF}"),
        );
        // Texts whose links could now lead to a moved text through an earlier candidate.
        let arriving = all_texts(Paging::default()).with_index("link", shift.new);
        let arriving_below = all_texts(Paging::default()).index_between(
            "link",
            format!("{}/", shift.new),
            format!("{}/\u{10FFFF}", shift.new),
        );
        let mut done: HashSet<ObjectId> = HashSet::new();
        for query in [exact, below, arriving, arriving_below] {
            for other in self.reads.query(&query).await? {
                let id = other.envelope.id;
                if moving.contains(&id) || !done.insert(id) {
                    continue;
                }
                let entry = entry_of(other)?;
                if let Some(doc) = relinked(&entry.document, &entry.path, &entry.path, shift) {
                    self.store_document(&entry, &entry.path, &doc).await?;
                }
            }
        }
        Ok(())
    }
}
