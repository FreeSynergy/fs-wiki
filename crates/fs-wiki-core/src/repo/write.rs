//! Create, edit and delete of texts.
//!
//! Pattern: Template method — one flow: check the path, build the object (envelope and index in
//! `record.rs`), write, read back. The public methods take the write lock; the `*_locked`
//! functions are the flows themselves, so `move` and `import` can combine them under one lock.
//! An edit keeps the stored envelope (id, `published`, IRI, attribution) and takes the name from
//! the new content (ADR 0032).

use fs_objects::ObjectId;
use fs_text::Document;

use super::handle::WikiRepo;
use super::revise::How;
use crate::error::{Error, Result};
use crate::feed::TextChange;
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
        self.write_text(id, None, path.as_str(), &doc, How::said("created"))
            .await?;
        self.feed.notify(&TextChange::Created(id));
        self.entry(id).await
    }

    pub(super) async fn edit_locked(&self, id: ObjectId, doc: Document) -> Result<TextEntry> {
        self.edit_said(id, doc, "edited").await
    }

    /// Edit with a summary (`editWith`; the plain edit says `edited`).
    pub(super) async fn edit_said(
        &self,
        id: ObjectId,
        mut doc: Document,
        summary: &str,
    ) -> Result<TextEntry> {
        let old = self.text(id).await?.ok_or(Error::NotFound(id))?;
        doc.id = id.as_uuid();
        self.write_text(id, Some(&old), &old.path, &doc, How::said(summary))
            .await?;
        self.feed.notify(&TextChange::Updated(id));
        self.entry(id).await
    }

    pub(super) async fn delete_locked(&self, id: ObjectId) -> Result<()> {
        self.text(id).await?.ok_or(Error::NotFound(id))?;
        self.objects.delete(id, &self.actor).await?;
        self.feed.notify(&TextChange::Deleted(id));
        Ok(())
    }

    /// The text `id`, which must exist (it was just written).
    pub(super) async fn entry(&self, id: ObjectId) -> Result<TextEntry> {
        self.text(id).await?.ok_or(Error::NotFound(id))
    }
}
