//! Reorder: where a child stands among its siblings.
//!
//! Pattern: Command — `reorder` writes the whole order of the parent (the ids of all siblings, in
//! the new order); a move across parents makes the page the last child of the new one.

use fs_objects::ObjectId;

use super::handle::WikiRepo;
use super::tree::{folder_of, parent_of_folder};
use crate::error::{Error, Result};
use crate::nodes::{Node, Placement};

impl WikiRepo {
    pub(super) async fn reorder_locked(
        &self,
        id: ObjectId,
        placement: Placement,
        anchor: Option<ObjectId>,
    ) -> Result<Node> {
        let folders = self.folders().await?;
        let parent = if let Some(f) = folders.iter().find(|f| f.id == id) {
            parent_of_folder(&folders, f)
        } else {
            let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
            folder_of(&folders, &text.path).map(|p| p.id)
        };
        let mut ids: Vec<ObjectId> = self.siblings(parent).await?.iter().map(|n| n.id).collect();
        let at = match placement {
            Placement::First => 0,
            Placement::Last => ids.len(),
            Placement::Before | Placement::After => {
                let anchor = anchor
                    .filter(|a| *a != id && ids.contains(a))
                    .ok_or_else(|| Error::Invalid {
                        issue: "anchor",
                        detail: "the anchor must be another child of the same parent".into(),
                    })?;
                ids.retain(|x| *x != id);
                let at = ids.iter().position(|x| *x == anchor).unwrap_or(0);
                if placement == Placement::Before {
                    at
                } else {
                    at + 1
                }
            }
        };
        ids.retain(|x| *x != id);
        ids.insert(at.min(ids.len()), id);
        self.order_put(parent, ids).await?;
        self.siblings(parent)
            .await?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or(Error::NotFound(id))
    }

    /// After a move from one parent to another: the text is the last child of the new one.
    pub(super) async fn settle_after_move(&self, id: ObjectId, old: &str, new: &str) -> Result<()> {
        let folders = self.folders().await?;
        let parent = |path: &str| folder_of(&folders, path).map(|f| f.id);
        let (before, after) = (parent(old), parent(new));
        if before == after {
            return Ok(());
        }
        let mut ids: Vec<ObjectId> = self.siblings(after).await?.iter().map(|n| n.id).collect();
        ids.retain(|x| *x != id);
        ids.push(id);
        self.order_put(after, ids).await
    }
}
