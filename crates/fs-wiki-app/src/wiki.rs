//! The wiki as the app holds it: the texts of the server and the file records of this app.
//!
//! Pattern: Facade (two providers presented as one) — `Texts` goes to the provider of the texts
//! (`RpcTexts`, the running `fs-wiki-server`), `Origins` to the provider of the records (`FileOrigins`,
//! a file of this app, not part of the contract). The sync and the import need both and are written
//! against `Texts + Origins`, so they do not know that the two live in different places.

use std::sync::Arc;

use async_trait::async_trait;
use fs_objects::ObjectId;
use fs_text::Document;
use fs_wiki_core::{
    Capabilities, FileOrigin, LinkEntry, Origins, Paging, Result, TextEntry, Texts,
};

/// `Texts` of one provider and `Origins` of another.
pub struct Wiki {
    texts: Arc<dyn Texts>,
    origins: Arc<dyn Origins>,
}

impl Wiki {
    /// The texts of `texts`, the file records of `origins`.
    pub fn new(texts: Arc<dyn Texts>, origins: Arc<dyn Origins>) -> Self {
        Self { texts, origins }
    }

    /// The provider of the texts, for what needs only them.
    pub fn texts(&self) -> Arc<dyn Texts> {
        self.texts.clone()
    }

    /// The provider of the records.
    pub fn origins_store(&self) -> Arc<dyn Origins> {
        self.origins.clone()
    }
}

#[async_trait]
impl Origins for Wiki {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        self.origins.origins().await
    }

    async fn set_origin(&self, origin: &FileOrigin) -> Result<()> {
        self.origins.set_origin(origin).await
    }
}

#[async_trait]
impl Texts for Wiki {
    fn capabilities(&self) -> Capabilities {
        self.texts.capabilities()
    }

    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        self.texts.get(id).await
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.texts.get_by_path(path).await
    }

    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.texts.list(prefix, page).await
    }

    async fn create(&self, path: &str, markdown: &str) -> Result<TextEntry> {
        self.texts.create(path, markdown).await
    }

    async fn edit(&self, id: ObjectId, document: Document) -> Result<TextEntry> {
        self.texts.edit(id, document).await
    }

    async fn edit_if(&self, id: ObjectId, revision: u64, document: Document) -> Result<TextEntry> {
        self.texts.edit_if(id, revision, document).await
    }

    async fn move_to(&self, id: ObjectId, path: &str) -> Result<TextEntry> {
        self.texts.move_to(id, path).await
    }

    async fn move_folder(&self, prefix: &str, path: &str) -> Result<Vec<TextEntry>> {
        self.texts.move_folder(prefix, path).await
    }

    /// Deleting a text here is on purpose: the records that name it are marked, so the file is
    /// not listed as missing at the next start.
    async fn delete(&self, id: ObjectId) -> Result<()> {
        self.texts.delete(id).await?;
        for record in self.origins.origins().await? {
            if record.text == id && !record.deleted {
                let marked = FileOrigin {
                    deleted: true,
                    ..record
                };
                self.origins.set_origin(&marked).await?;
            }
        }
        Ok(())
    }

    async fn find(&self, term: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.texts.find(term, page).await
    }

    async fn links(&self, id: ObjectId) -> Result<Vec<LinkEntry>> {
        self.texts.links(id).await
    }

    async fn backlinks(&self, path: &str) -> Result<Vec<LinkEntry>> {
        self.texts.backlinks(path).await
    }

    async fn import_text(&self, path: &str, media_type: &str, text: &str) -> Result<TextEntry> {
        self.texts.import_text(path, media_type, text).await
    }

    async fn export_text(&self, id: ObjectId, media_type: &str) -> Result<String> {
        self.texts.export_text(id, media_type).await
    }
}
