//! `Texts` over the socket: the sixteen methods of the contract (the fourteen of `Texts`, `capabilities` and `subscribe`).
//!
//! Pattern: Adapter — each method is one call of `texts.capnp`; the arguments are made owned here,
//! because the call runs on the thread of the connection. `list` and `find` read as many pages as
//! `Paging` needs (`calls/pages.rs`); `page` reads from the start as well and drops what comes before its place, each page a call of its own with its own deadline. The report is the server's own: every method of the trait is
//! kept, so nothing the server offers is withheld.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_objects::ObjectId;
use fs_objects::StoreId;
use fs_text::Document;
use fs_wiki_core::{LinkEntry, Paging, Place, Result, TextEntry, TextPage, TextQuery, Texts};

use crate::calls::pages::{collect, collect_after};
use crate::calls::writes::document_json;
use crate::calls::{reads, writes};
use crate::RpcTexts;

#[async_trait]
impl Texts for RpcTexts {
    fn capabilities(&self) -> Capabilities {
        self.report()
    }

    fn identity(&self) -> Option<StoreId> {
        RpcTexts::identity(self)
    }

    async fn page(
        &self,
        query: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        match query {
            TextQuery::Prefix(prefix) => {
                collect_after(after, size, |cursor| {
                    let prefix = prefix.clone();
                    self.call(
                        move |r| async move { reads::list_page(&r.texts, prefix, cursor).await },
                    )
                })
                .await
            }
            TextQuery::Term(term) => {
                collect_after(after, size, |cursor| {
                    let term = term.clone();
                    self.call(
                        move |r| async move { reads::find_page(&r.texts, term, cursor).await },
                    )
                })
                .await
            }
        }
    }

    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        self.call(move |r| async move { reads::get(&r.texts, id).await })
            .await
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        let path = path.to_string();
        self.call(move |r| async move { reads::get_by_path(&r.texts, path).await })
            .await
    }

    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        let prefix = prefix.to_string();
        collect(page, |cursor| {
            let prefix = prefix.clone();
            self.call(move |r| async move { reads::list_page(&r.texts, prefix, cursor).await })
        })
        .await
    }

    async fn create(&self, path: &str, markdown: &str) -> Result<TextEntry> {
        let (path, markdown) = (path.to_string(), markdown.to_string());
        self.call(move |r| async move { writes::create(&r.texts, path, markdown).await })
            .await
    }

    async fn edit(&self, id: ObjectId, document: Document) -> Result<TextEntry> {
        let document = document_json(&document)?;
        self.call(move |r| async move { writes::edit(&r.texts, id, document).await })
            .await
    }

    async fn edit_if(&self, id: ObjectId, revision: u64, document: Document) -> Result<TextEntry> {
        let document = document_json(&document)?;
        self.call(move |r| async move { writes::edit_if(&r.texts, id, revision, document).await })
            .await
    }

    async fn move_to(&self, id: ObjectId, path: &str) -> Result<TextEntry> {
        let path = path.to_string();
        self.call(move |r| async move { writes::move_to(&r.texts, id, path).await })
            .await
    }

    async fn move_folder(&self, prefix: &str, path: &str) -> Result<Vec<TextEntry>> {
        let (prefix, path) = (prefix.to_string(), path.to_string());
        self.call(move |r| async move { writes::move_folder(&r.texts, prefix, path).await })
            .await
    }

    async fn delete(&self, id: ObjectId) -> Result<()> {
        self.call(move |r| async move { writes::delete(&r.texts, id).await })
            .await
    }

    async fn find(&self, term: &str, page: Paging) -> Result<Vec<TextEntry>> {
        let term = term.to_string();
        collect(page, |cursor| {
            let term = term.clone();
            self.call(move |r| async move { reads::find_page(&r.texts, term, cursor).await })
        })
        .await
    }

    async fn links(&self, id: ObjectId) -> Result<Vec<LinkEntry>> {
        self.call(move |r| async move { reads::links(&r.texts, id).await })
            .await
    }

    async fn backlinks(&self, path: &str) -> Result<Vec<LinkEntry>> {
        let path = path.to_string();
        self.call(move |r| async move { reads::backlinks(&r.texts, path).await })
            .await
    }

    async fn import_text(&self, path: &str, media_type: &str, text: &str) -> Result<TextEntry> {
        let (path, media_type, text) = (path.to_string(), media_type.to_string(), text.to_string());
        self.call(
            move |r| async move { writes::import_text(&r.texts, path, media_type, text).await },
        )
        .await
    }

    async fn export_text(&self, id: ObjectId, media_type: &str) -> Result<String> {
        let media_type = media_type.to_string();
        self.call(move |r| async move { reads::export_text(&r.texts, id, media_type).await })
            .await
    }
}
