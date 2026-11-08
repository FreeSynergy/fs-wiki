//! `Texts` implemented by `WikiRepo`.
//!
//! Pattern: Facade — each trait method checks its input, takes the write lock where it writes
//! and forwards to the flow behind it, so the contract stays one thin file.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_objects::{ObjectId, StoreId};
use fs_text::Document;

use super::handle::WikiRepo;
use crate::error::{Error, Result};
use crate::history::{Expect, RevisionContent, RevisionPage};
use crate::media::TextFormat;
use crate::nodes::{Node, NodePage, Placement};
use crate::page::{Place, TextPage, TextQuery};
use crate::path::TextPath;
use crate::redirect::Resolution;
use crate::search::{SearchPage, SearchPlace, SearchQuery};
use crate::texts::{LinkEntry, Paging, TextEntry, Texts};
use crate::trash::{TrashItem, TrashPage, TrashPlace};

#[async_trait]
impl Texts for WikiRepo {
    fn capabilities(&self) -> Capabilities {
        super::caps::capabilities(self.purger.is_some())
    }
    fn identity(&self) -> Option<StoreId> {
        self.info.as_ref().map(|info| info.identity())
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        self.text(id).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.text_at(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.list_texts(prefix, page).await
    }
    async fn page(
        &self,
        query: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        self.page_of(query, after, size).await
    }
    async fn create(&self, path: &str, markdown: &str) -> Result<TextEntry> {
        let path = TextPath::parse(path)?;
        let _write = self.write_lock.lock().await;
        self.create_locked(&path, TextFormat::Markdown.import(markdown))
            .await
    }
    async fn edit(&self, id: ObjectId, document: Document) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.edit_locked(id, document).await
    }
    async fn edit_if(&self, id: ObjectId, revision: u64, document: Document) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        let found = self.text(id).await?.ok_or(Error::NotFound(id))?.revision;
        if found != revision {
            return Err(Error::Conflict {
                id,
                expected: revision,
                found,
            });
        }
        self.edit_locked(id, document).await
    }
    async fn move_to(&self, id: ObjectId, path: &str) -> Result<TextEntry> {
        let path = TextPath::parse(path)?;
        let _write = self.write_lock.lock().await;
        let old = self.text(id).await?.map(|t| t.path);
        let moved = self.move_locked(id, &path).await?;
        if let Some(old) = old {
            self.settle_after_move(id, &old, &moved.path).await?;
        }
        Ok(moved)
    }
    async fn move_folder(&self, prefix: &str, path: &str) -> Result<Vec<TextEntry>> {
        let prefix = TextPath::parse(prefix)?;
        let path = TextPath::parse(path)?;
        let _write = self.write_lock.lock().await;
        self.move_folder_locked(&prefix, &path).await
    }
    async fn delete(&self, id: ObjectId) -> Result<()> {
        let _write = self.write_lock.lock().await;
        self.delete_locked(id).await
    }
    async fn find(&self, term: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.find_texts(term, page).await
    }
    async fn links(&self, id: ObjectId) -> Result<Vec<LinkEntry>> {
        self.links_of(id).await
    }
    async fn backlinks(&self, path: &str) -> Result<Vec<LinkEntry>> {
        self.backlinks_to(path).await
    }
    async fn import_text(&self, path: &str, media_type: &str, text: &str) -> Result<TextEntry> {
        let format = TextFormat::from_media_type(media_type)?;
        let path = TextPath::parse(path)?;
        let _write = self.write_lock.lock().await;
        self.import_locked(&path, format, text).await
    }
    async fn export_text(&self, id: ObjectId, media_type: &str) -> Result<String> {
        self.export_of(id, TextFormat::from_media_type(media_type)?)
            .await
    }
    async fn edit_with(
        &self,
        id: ObjectId,
        expect: Expect,
        document: Document,
        summary: &str,
    ) -> Result<TextEntry> {
        Self::check_summary(summary)?;
        let _write = self.write_lock.lock().await;
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Self::check_expect(&text, expect)?;
        let summary = if summary.is_empty() {
            "edited"
        } else {
            summary
        };
        self.edit_said(id, document, summary).await
    }
    async fn history(
        &self,
        id: ObjectId,
        before: Option<u64>,
        size: usize,
    ) -> Result<RevisionPage> {
        self.history_of(id, before, size).await
    }
    async fn get_revision(&self, id: ObjectId, number: u64) -> Result<Option<RevisionContent>> {
        self.revision_of(id, number).await
    }
    async fn restore_revision(
        &self,
        id: ObjectId,
        number: u64,
        expect: Expect,
    ) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.restore_locked(id, number, expect).await
    }
    async fn purge_revision(&self, id: ObjectId, number: u64) -> Result<()> {
        let _write = self.write_lock.lock().await;
        self.purge_revision_locked(id, number).await
    }
    async fn trash(&self, id: ObjectId) -> Result<TrashItem> {
        let _write = self.write_lock.lock().await;
        self.trash_locked(id).await
    }
    async fn list_trash(&self, after: Option<&TrashPlace>, size: usize) -> Result<TrashPage> {
        self.trash_page(after, size).await
    }
    async fn restore_from_trash(&self, id: ObjectId, path: &str) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.restore_trashed_locked(id, path).await
    }
    async fn purge(&self, id: ObjectId) -> Result<()> {
        let _write = self.write_lock.lock().await;
        self.purge_locked(id).await
    }
    async fn create_folder(
        &self,
        parent: Option<ObjectId>,
        name: &str,
        path: &str,
    ) -> Result<Node> {
        let _write = self.write_lock.lock().await;
        let folder = self.create_folder_locked(parent, name, path).await?;
        self.folder_node(folder.id).await
    }
    async fn rename_folder(&self, id: ObjectId, name: &str) -> Result<Node> {
        let _write = self.write_lock.lock().await;
        self.rename_folder_locked(id, name).await?;
        self.folder_node(id).await
    }
    async fn delete_folder(&self, id: ObjectId) -> Result<()> {
        let _write = self.write_lock.lock().await;
        self.delete_folder_locked(id).await
    }
    async fn get_folder(&self, id: ObjectId) -> Result<Option<Node>> {
        match self.folder_node(id).await {
            Err(Error::NoSuchNode(_)) => Ok(None),
            other => other.map(Some),
        }
    }
    async fn children(
        &self,
        parent: Option<ObjectId>,
        offset: usize,
        size: usize,
    ) -> Result<NodePage> {
        self.children_page(parent, offset, size).await
    }
    async fn reorder(
        &self,
        id: ObjectId,
        placement: Placement,
        anchor: Option<ObjectId>,
    ) -> Result<Node> {
        let _write = self.write_lock.lock().await;
        self.reorder_locked(id, placement, anchor).await
    }
    async fn set_template(
        &self,
        id: ObjectId,
        template: bool,
        expect: Expect,
    ) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.set_template_locked(id, template, expect).await
    }
    async fn create_from_template(&self, template: ObjectId, path: &str) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.copy_template_locked(template, path).await
    }
    async fn set_redirect(&self, id: ObjectId, target: &str, expect: Expect) -> Result<TextEntry> {
        let _write = self.write_lock.lock().await;
        self.set_redirect_locked(id, target, expect).await
    }
    async fn resolve(&self, path: &str) -> Result<Resolution> {
        self.resolve_path(path).await
    }
    async fn search(
        &self,
        query: &SearchQuery,
        after: Option<&SearchPlace>,
        size: usize,
    ) -> Result<SearchPage> {
        self.search_page(query, after, size).await
    }
}
