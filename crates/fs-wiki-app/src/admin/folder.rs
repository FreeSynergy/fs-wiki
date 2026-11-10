//! `Folder` for the Admin frontend: spaces and folders of the tree.
//!
//! Pattern: Strategy by command, as `page.rs`. The list walks the tree from the top (`children`) and keeps spaces and
//! folders; `move` carries the folder and everything below it (`move_folder` takes the path, so it is read first).

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_wiki_core::describe::is_supported;
use fs_wiki_core::{Node, NodeKind, Placement};

use super::ctx::{id, need, one, optional_id, unsupported, Ctx, LIST_LIMIT};
use super::records;

/// The spaces and folders.
pub struct FolderBackend(pub Ctx);

impl FolderBackend {
    fn all(&self) -> Result<Vec<Node>, BackendError> {
        let mut out = Vec::new();
        let mut todo = vec![None];
        while let Some(parent) = todo.pop() {
            let page = self.0.block(self.0.texts.children(parent, 0, LIST_LIMIT))?;
            for n in page.items.into_iter().filter(|n| n.kind != NodeKind::Page) {
                todo.push(Some(n.id));
                out.push(n);
            }
            if out.len() >= LIST_LIMIT {
                break;
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn placement(word: &str) -> Result<Placement, BackendError> {
        Ok(match word {
            "first" => Placement::First,
            "last" => Placement::Last,
            "before" => Placement::Before,
            "after" => Placement::After,
            _ => return Err(super::ctx::not_a_number("placement")),
        })
    }
}

impl Backend for FolderBackend {
    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        let name = one(q, "name").unwrap_or_default().to_lowercase();
        let parent = optional_id(q, "parent")?;
        Ok(self
            .all()?
            .iter()
            .filter(|n| name.is_empty() || n.name.to_lowercase().contains(&name))
            .filter(|n| parent.is_none() || n.parent == parent)
            .map(records::folder)
            .collect())
    }

    fn run(&self, name: &str, input: &Values) -> Result<Outcome, BackendError> {
        if !is_supported("Folder", name) {
            return Err(unsupported(name));
        }
        let t = &self.0.texts;
        match name {
            "create" => {
                let n = self.0.block(t.create_folder(optional_id(input, "parent")?, need(input, "name")?, one(input, "path").unwrap_or_default()))?;
                Ok(records::outcome(n.id))
            }
            "rename" => {
                let n = self.0.block(t.rename_folder(id(input, "id")?, need(input, "name")?))?;
                Ok(records::outcome(n.id))
            }
            "move" => {
                let folder = id(input, "id")?;
                let node = self.0.block(t.get_folder(folder))?.ok_or(BackendError::NotFound)?;
                self.0.block(t.move_folder(&node.path, need(input, "path")?))?;
                Ok(records::outcome(folder))
            }
            "reorder" => {
                let at = Self::placement(need(input, "placement")?)?;
                let n = self.0.block(t.reorder(id(input, "id")?, at, optional_id(input, "anchor")?))?;
                Ok(records::outcome(n.id))
            }
            "delete" => {
                self.0.block(t.delete_folder(id(input, "id")?))?;
                Ok(Outcome::default())
            }
            other => Err(unsupported(other)),
        }
    }
}
