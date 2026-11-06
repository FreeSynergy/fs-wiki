//! Create, edit and delete of texts.
//!
//! Pattern: Template method — one flow: check the path, build the object (envelope and index in
//! `record.rs`), write, read back. The public methods take the write lock; the `*_locked`
//! functions are the flows themselves, so `move` and `import` can combine them under one lock.
//! An edit keeps the stored envelope (id, `published`, IRI, attribution) and takes the name from
//! the new content (ADR 0032).

use fs_store::ObjectId;
use fs_text::Document;

use super::handle::WikiRepo;
use crate::error::{Error, Result};
use crate::path::TextPath;
use crate::texts::TextEntry;

impl WikiRepo {
    /// `Err(PathTaken)` unless `path` is free or held by `except`.
    pub(super) async fn ensure_free(
        &self,
        path: &TextPath,
        except: Option<ObjectId>,
    ) -> Result<()> {
        match self.text_at(path.as_str()).await? {
            Some(t) if Some(t.id) != except => Err(Error::PathTaken(path.as_str().into())),
            _ => Ok(()),
        }
    }

    pub(super) async fn create_locked(
        &self,
        path: &TextPath,
        mut doc: Document,
    ) -> Result<TextEntry> {
        self.ensure_free(path, None).await?;
        let id = ObjectId::new();
        doc.id = id.as_uuid();
        let obj = self.build_object(id, None, path.as_str(), &doc)?;
        self.objects.put(obj, &self.actor).await?;
        self.entry(id).await
    }

    pub(super) async fn edit_locked(&self, id: ObjectId, mut doc: Document) -> Result<TextEntry> {
        let old = self.text(id).await?.ok_or(Error::NotFound(id))?;
        doc.id = id.as_uuid();
        let obj = self.build_object(id, Some(&old.envelope), &old.path, &doc)?;
        self.objects.put(obj, &self.actor).await?;
        self.entry(id).await
    }

    pub(super) async fn delete_locked(&self, id: ObjectId) -> Result<()> {
        self.text(id).await?.ok_or(Error::NotFound(id))?;
        self.objects.delete(id, &self.actor).await?;
        Ok(())
    }

    /// The text `id`, which must exist (it was just written).
    pub(super) async fn entry(&self, id: ObjectId) -> Result<TextEntry> {
        self.text(id).await?.ok_or(Error::NotFound(id))
    }
}
