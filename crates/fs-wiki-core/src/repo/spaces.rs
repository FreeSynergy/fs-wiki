//! Spaces and folders: the nodes that own a path prefix.
//!
//! Pattern: Repository — a folder is a record of its own (type `WikiFolder`, written as the
//! bookkeeping actor): a display name, the prefix it owns, and whether it is a top-level space. The
//! texts under the prefix are not changed by it; a page's parent is derived from the longest prefix
//! (`tree.rs`). There are few folders, so they are read all at once.

use fs_objects::{Envelope, ObjectId, Query, StoredObject};
use serde::{Deserialize, Serialize};

use super::handle::WikiRepo;
use crate::error::{Error, Result};
use crate::limits::MAX_FOLDER_NAME_BYTES;
use crate::path::TextPath;
use crate::reason::Reason;

const FOLDER_TYPE: &str = "WikiFolder";
const FOLDER_CONTEXT: &str = "wiki-folders";

/// A space or folder as stored.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Folder {
    #[serde(skip, default = "ObjectId::new")]
    pub id: ObjectId,
    pub name: String,
    pub path: String,
    pub space: bool,
}

impl Folder {
    /// Whether `path` lies strictly below this folder.
    pub(super) fn owns(&self, path: &str) -> bool {
        path.len() > self.path.len() + 1
            && path.starts_with(&self.path)
            && path.as_bytes()[self.path.len()] == b'/'
    }
}

/// `name` as the last segment of a path: ASCII letters and digits, anything between them one `-`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn invalid(issue: &'static str, detail: &str) -> Error {
    Error::Invalid {
        issue,
        detail: detail.into(),
    }
}

impl WikiRepo {
    /// Every folder, in path order.
    pub(super) async fn folders(&self) -> Result<Vec<Folder>> {
        let q = Query::new().of_type(FOLDER_TYPE).in_context(FOLDER_CONTEXT);
        let mut all = vec![];
        for o in self.reads.query(&q).await? {
            let id = o.envelope.id;
            all.push(Folder {
                id,
                ..serde_json::from_value(o.content)?
            });
        }
        all.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(all)
    }

    pub(super) async fn folder(&self, id: ObjectId) -> Result<Folder> {
        match self.objects.get(id).await? {
            Some(o) if o.envelope.object_type == FOLDER_TYPE => Ok(Folder {
                id,
                ..serde_json::from_value(o.content)?
            }),
            _ => Err(Error::NoSuchNode(id)),
        }
    }

    async fn put_folder(&self, folder: &Folder) -> Result<()> {
        let mut env = Envelope::new(FOLDER_TYPE, folder.name.clone());
        env.id = folder.id;
        env.context = Some(FOLDER_CONTEXT.into());
        let object = StoredObject::new(env, "application/json", serde_json::to_value(folder)?);
        self.objects.put(object, &self.aux_actor()).await?;
        Ok(())
    }

    pub(super) async fn create_folder_locked(
        &self,
        parent: Option<ObjectId>,
        name: &str,
        path: &str,
    ) -> Result<Folder> {
        if name.is_empty() {
            return Err(invalid("name-empty", "a folder needs a name"));
        }
        if name.len() > MAX_FOLDER_NAME_BYTES {
            return Err(invalid(
                "name-too-long",
                "a folder name is at most 255 bytes",
            ));
        }
        if name.contains('/') {
            return Err(invalid("name-has-slash", "a folder name has no '/'"));
        }
        let above = match parent {
            Some(p) => Some(self.folder(p).await?),
            None => None,
        };
        let path = Self::folder_path(above.as_ref(), name, path)?;
        if self.folders().await?.iter().any(|f| f.path == path) {
            return Err(Error::PathTaken(path));
        }
        let folder = Folder {
            id: ObjectId::new(),
            name: name.into(),
            path,
            space: above.is_none(),
        };
        self.put_folder(&folder).await?;
        Ok(folder)
    }

    /// The prefix a new folder owns: the given path checked, or the slug of the name below its parent.
    fn folder_path(parent: Option<&Folder>, name: &str, path: &str) -> Result<String> {
        if path.is_empty() {
            let slug = slug(name);
            if slug.is_empty() {
                return Err(Error::InvalidPath {
                    path: name.into(),
                    reason: Reason::EmptySlug,
                });
            }
            return Ok(parent.map_or(slug.clone(), |p| format!("{}/{slug}", p.path)));
        }
        let path = TextPath::parse(path)?;
        if parent.is_some_and(|p| !p.owns(path.as_str())) {
            return Err(Error::InvalidPath {
                path: path.as_str().into(),
                reason: Reason::OutsideParent,
            });
        }
        Ok(path.as_str().into())
    }

    pub(super) async fn rename_folder_locked(&self, id: ObjectId, name: &str) -> Result<Folder> {
        let mut folder = self.folder(id).await?;
        if name.is_empty() || name.len() > MAX_FOLDER_NAME_BYTES || name.contains('/') {
            return Err(invalid(
                "name",
                "a folder name is 1 to 255 bytes without '/'",
            ));
        }
        folder.name = name.into();
        self.put_folder(&folder).await?;
        Ok(folder)
    }

    pub(super) async fn delete_folder_locked(&self, id: ObjectId) -> Result<()> {
        let folder = self.folder(id).await?;
        let below = self.pages_below(Some(&folder.path), 1).await?;
        let inside = self.folders().await?.iter().any(|f| folder.owns(&f.path));
        if !below.is_empty() || inside || self.trash_below(&folder.path).await? {
            return Err(Error::NotEmpty(id));
        }
        self.objects.delete(id, &self.aux_actor()).await?;
        self.order_drop(Some(id)).await
    }

    /// A folder move carries the folder nodes at and below `old` to `new`; their ids, and so their
    /// order lists, stay. Repeating it changes nothing.
    pub(super) async fn move_folder_nodes(&self, old: &str, new: &str) -> Result<usize> {
        let mut moved = 0;
        for mut folder in self.folders().await? {
            let rest = folder.path.strip_prefix(old);
            if rest.is_some_and(|r| r.is_empty() || r.starts_with('/')) {
                folder.path = format!("{new}{}", rest.unwrap_or(""));
                self.put_folder(&folder).await?;
                moved += 1;
            }
        }
        Ok(moved)
    }
}
