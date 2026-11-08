//! The tree of spaces, folders and pages, and the order of siblings.
//!
//! Pattern: Read model + Command — a node's parent is derived from paths (the folder with the longest
//! prefix; a space has none), so nothing but the order is stored for a page. `siblings` is the one
//! place that assembles the children of a parent; `children`, `get_folder` and `reorder` read it.
//! It costs the texts below the parent (the store still hands back each whole record; only the parsing of the document is skipped), not the whole wiki,
//! unless the parent is the top level.

use fs_objects::ObjectId;

use super::handle::WikiRepo;
use super::order::arranged;
use super::read::all_texts;
use super::spaces::Folder;
use crate::error::{Error, Result};
use crate::limits::MAX_CHILDREN_PAGE;
use crate::nodes::{Node, NodeKind, NodePage};
use crate::texts::Paging;

/// A text as the tree needs it: the document is not parsed.
pub(super) struct Light {
    pub id: ObjectId,
    pub path: String,
    pub name: String,
    pub template: bool,
    pub redirect: bool,
}

/// The folder a path belongs to: the one with the longest prefix.
pub(super) fn folder_of<'a>(folders: &'a [Folder], path: &str) -> Option<&'a Folder> {
    folders
        .iter()
        .filter(|f| f.owns(path))
        .max_by_key(|f| f.path.len())
}

pub(super) fn parent_of_folder(folders: &[Folder], f: &Folder) -> Option<ObjectId> {
    if f.space {
        return None;
    }
    folder_of(folders, &f.path).map(|p| p.id)
}

fn node_of_folder(folders: &[Folder], f: &Folder) -> Node {
    Node {
        id: f.id,
        kind: if f.space {
            NodeKind::Space
        } else {
            NodeKind::Folder
        },
        name: f.name.clone(),
        path: f.path.clone(),
        parent: parent_of_folder(folders, f),
        position: 0,
        template: false,
        redirect: false,
    }
}

fn node_of_page(folders: &[Folder], l: &Light) -> Node {
    Node {
        id: l.id,
        kind: NodeKind::Page,
        name: l.name.clone(),
        path: l.path.clone(),
        parent: folder_of(folders, &l.path).map(|p| p.id),
        position: 0,
        template: l.template,
        redirect: l.redirect,
    }
}

impl WikiRepo {
    /// The texts below `prefix/` (all texts for none), documents not parsed; at most `limit`.
    pub(super) async fn pages_below(
        &self,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Light>> {
        const CHUNK: usize = 500;
        let mut out: Vec<Light> = vec![];
        loop {
            let mut q = all_texts(Paging::new(CHUNK, out.len()));
            if let Some(p) = prefix {
                q = q.index_between("path", format!("{p}/"), format!("{p}/\u{10FFFF}"));
            }
            let chunk = self.reads.query(&q).await?;
            let n = chunk.len();
            for o in chunk {
                out.push(Light {
                    id: o.envelope.id,
                    path: o.content["path"].as_str().unwrap_or("").to_string(),
                    name: o.envelope.name.clone(),
                    template: o.content["template"].as_bool().unwrap_or(false),
                    redirect: !o.content["redirect_to"].as_str().unwrap_or("").is_empty(),
                });
            }
            if n < CHUNK || out.len() >= limit {
                out.truncate(limit);
                return Ok(out);
            }
        }
    }

    /// The children of `parent` in order, with their positions.
    pub(super) async fn siblings(&self, parent: Option<ObjectId>) -> Result<Vec<Node>> {
        let folders = self.folders().await?;
        let prefix = match parent {
            Some(p) => Some(
                folders
                    .iter()
                    .find(|f| f.id == p)
                    .ok_or(Error::NoSuchNode(p))?
                    .path
                    .clone(),
            ),
            None => None,
        };
        let mut nodes: Vec<Node> = folders
            .iter()
            .filter(|f| parent_of_folder(&folders, f) == parent)
            .map(|f| node_of_folder(&folders, f))
            .collect();
        for l in self.pages_below(prefix.as_deref(), usize::MAX).await? {
            let node = node_of_page(&folders, &l);
            if node.parent == parent {
                nodes.push(node);
            }
        }
        let ids = arranged(
            nodes.iter().map(|n| n.id).collect(),
            &self.order_get(parent).await?,
        );
        let mut ordered: Vec<Node> = vec![];
        for (position, id) in (0u32..).zip(ids) {
            if let Some(i) = nodes.iter().position(|n| n.id == id) {
                let mut node = nodes.swap_remove(i);
                node.position = position;
                ordered.push(node);
            }
        }
        Ok(ordered)
    }

    pub(super) async fn children_page(
        &self,
        parent: Option<ObjectId>,
        offset: usize,
        size: usize,
    ) -> Result<NodePage> {
        let all = self.siblings(parent).await?;
        let size = size.clamp(1, MAX_CHILDREN_PAGE);
        let more = all.len() > offset.saturating_add(size);
        let items = all.into_iter().skip(offset).take(size).collect();
        Ok(NodePage { items, more })
    }

    /// The space or folder `id` as a node, with its position among its siblings.
    pub(super) async fn folder_node(&self, id: ObjectId) -> Result<Node> {
        let folders = self.folders().await?;
        let f = folders
            .iter()
            .find(|f| f.id == id)
            .ok_or(Error::NoSuchNode(id))?;
        let parent = parent_of_folder(&folders, f);
        let mut node = node_of_folder(&folders, f);
        if let Some(n) = self
            .siblings(parent)
            .await?
            .into_iter()
            .find(|n| n.id == id)
        {
            node.position = n.position;
        }
        Ok(node)
    }
}
