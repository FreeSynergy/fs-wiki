//! `Texts` implemented by `WikiRepo`.
//!
//! Pattern: Facade — each trait method checks its input, takes the write lock where it writes
//! and forwards to the flow behind it, so the contract stays one thin file.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_store::ObjectId;
use fs_text::Document;

use super::handle::WikiRepo;
use crate::error::{Error, Result};
use crate::media::TextFormat;
use crate::path::TextPath;
use crate::texts::{LinkEntry, Paging, TextEntry, Texts};

#[async_trait]
impl Texts for WikiRepo {
    fn capabilities(&self) -> Capabilities {
        capabilities()
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
        self.move_locked(id, &path).await
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
}

/// The store-backed provider offers every method of the contract.
fn capabilities() -> Capabilities {
    let mut caps = Capabilities::new("fs-wiki");
    for id in [
        "text.create",
        "text.edit",
        "text.move",
        "text.delete",
        "text.read",
        "text.find",
        "link.read",
        "text.import",
        "text.export",
        "text.subscribe",
    ] {
        caps.supported(id);
    }
    caps.supported("text.subscribe");
    caps
}
