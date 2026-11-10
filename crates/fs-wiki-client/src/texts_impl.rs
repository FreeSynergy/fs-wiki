//! `Texts` over the socket: every method of the contract, texts 0.1 and 0.2 (`capabilities` and `subscribe` apart).
//!
//! Pattern: Adapter — each method is one call of `texts.capnp`; the arguments are made owned here,
//! because the call runs on the thread of the connection. `list` and `find` read as many pages as
//! `Paging` needs (`calls/pages.rs`); `page` hands the server the cursor of its place, each page a call of its own with its own deadline. The report is the server's own: every method of the trait is
//! kept, so nothing the server offers is withheld.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_objects::ObjectId;
use fs_objects::StoreId;
use fs_text::Document;
use fs_wiki_core::{
    Expect, LinkEntry, Node, NodePage, Paging, Place, Placement, Resolution, Result,
    RevisionContent, RevisionPage, SearchPage, SearchPlace, SearchQuery, TextEntry, TextPage,
    TextQuery, Texts, TrashItem, TrashPage, TrashPlace,
};

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

    // ---- texts 0.2 (the calls are in `texts_v2.rs`) ----

    async fn edit_with(
        &self,
        id: ObjectId,
        expect: Expect,
        document: Document,
        summary: &str,
    ) -> Result<TextEntry> {
        self.edit_with_call(id, expect, document, summary).await
    }

    async fn history(
        &self,
        id: ObjectId,
        before: Option<u64>,
        size: usize,
    ) -> Result<RevisionPage> {
        self.history_call(id, before, size).await
    }

    async fn get_revision(&self, id: ObjectId, number: u64) -> Result<Option<RevisionContent>> {
        self.get_revision_call(id, number).await
    }

    async fn restore_revision(
        &self,
        id: ObjectId,
        number: u64,
        expect: Expect,
    ) -> Result<TextEntry> {
        self.restore_revision_call(id, number, expect).await
    }

    async fn purge_revision(&self, id: ObjectId, number: u64) -> Result<()> {
        self.purge_revision_call(id, number).await
    }

    async fn trash(&self, id: ObjectId) -> Result<TrashItem> {
        self.trash_call(id).await
    }

    async fn list_trash(&self, after: Option<&TrashPlace>, size: usize) -> Result<TrashPage> {
        self.list_trash_call(after, size).await
    }

    async fn restore_from_trash(&self, id: ObjectId, path: &str) -> Result<TextEntry> {
        self.restore_from_trash_call(id, path).await
    }

    async fn purge(&self, id: ObjectId) -> Result<()> {
        self.purge_call(id).await
    }

    async fn create_folder(
        &self,
        parent: Option<ObjectId>,
        name: &str,
        path: &str,
    ) -> Result<Node> {
        self.create_folder_call(parent, name, path).await
    }

    async fn rename_folder(&self, id: ObjectId, name: &str) -> Result<Node> {
        self.rename_folder_call(id, name).await
    }

    async fn delete_folder(&self, id: ObjectId) -> Result<()> {
        self.delete_folder_call(id).await
    }

    async fn get_folder(&self, id: ObjectId) -> Result<Option<Node>> {
        self.get_folder_call(id).await
    }

    async fn children(
        &self,
        parent: Option<ObjectId>,
        offset: usize,
        size: usize,
    ) -> Result<NodePage> {
        self.children_call(parent, offset, size).await
    }

    async fn reorder(
        &self,
        id: ObjectId,
        placement: Placement,
        anchor: Option<ObjectId>,
    ) -> Result<Node> {
        self.reorder_call(id, placement, anchor).await
    }

    async fn set_template(
        &self,
        id: ObjectId,
        template: bool,
        expect: Expect,
    ) -> Result<TextEntry> {
        self.set_template_call(id, template, expect).await
    }

    async fn create_from_template(&self, template: ObjectId, path: &str) -> Result<TextEntry> {
        self.create_from_template_call(template, path).await
    }

    async fn set_redirect(&self, id: ObjectId, target: &str, expect: Expect) -> Result<TextEntry> {
        self.set_redirect_call(id, target, expect).await
    }

    async fn resolve(&self, path: &str) -> Result<Resolution> {
        self.resolve_call(path).await
    }

    async fn search(
        &self,
        query: &SearchQuery,
        after: Option<&SearchPlace>,
        size: usize,
    ) -> Result<SearchPage> {
        self.search_call(query, after, size).await
    }
}
