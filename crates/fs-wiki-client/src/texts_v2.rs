//! The calls of texts 0.2 as methods of the connection; `texts_impl.rs` hands the trait methods to them.
//!
//! Pattern: Adapter, as `texts_impl.rs` (split for the file limit): each method is one call of `texts.capnp`, the arguments
//! made owned. A list that continues after a place is read from the start and cut there (`calls/v2_pages.rs`).

use fs_objects::ObjectId;
use fs_text::Document;
use fs_wiki_core::{
    Expect, Node, NodePage, Placement, Resolution, Result, RevisionContent, RevisionPage,
    SearchHit, SearchPage, SearchPlace, SearchQuery, TextEntry, TrashItem, TrashPage, TrashPlace,
};

use crate::calls::v2_pages;
use crate::calls::writes::document_json;
use crate::calls::{v2, v2_history, v2_nodes};
use crate::RpcTexts;

impl RpcTexts {
    pub(crate) async fn edit_with_call(
        &self,
        id: ObjectId,
        expect: Expect,
        document: Document,
        summary: &str,
    ) -> Result<TextEntry> {
        let (document, summary) = (document_json(&document)?, summary.to_string());
        self.call(
            move |r| async move { v2::edit_with(&r.texts, id, expect, document, summary).await },
        )
        .await
    }

    pub(crate) async fn history_call(
        &self,
        id: ObjectId,
        before: Option<u64>,
        size: usize,
    ) -> Result<RevisionPage> {
        let page =
            self.call(move |r| async move {
                v2_history::history(&r.texts, id, v2_pages::READ_LIMIT).await
            })
            .await?;
        let kept: Vec<_> = page
            .items
            .into_iter()
            .filter(|i| before.is_none_or(|b| i.number < b))
            .collect();
        let (items, more) = v2_pages::cut(kept, page.more, None::<fn(&_) -> bool>, size);
        Ok(RevisionPage { items, more })
    }

    pub(crate) async fn get_revision_call(
        &self,
        id: ObjectId,
        number: u64,
    ) -> Result<Option<RevisionContent>> {
        self.call(move |r| async move { v2_history::get_revision(&r.texts, id, number).await })
            .await
    }

    pub(crate) async fn restore_revision_call(
        &self,
        id: ObjectId,
        number: u64,
        expect: Expect,
    ) -> Result<TextEntry> {
        self.call(move |r| async move {
            v2_history::restore_revision(&r.texts, id, number, expect).await
        })
        .await
    }

    pub(crate) async fn purge_revision_call(&self, id: ObjectId, number: u64) -> Result<()> {
        self.call(move |r| async move { v2_history::purge_revision(&r.texts, id, number).await })
            .await
    }

    pub(crate) async fn trash_call(&self, id: ObjectId) -> Result<TrashItem> {
        self.call(move |r| async move { v2_history::trash(&r.texts, id).await })
            .await
    }

    pub(crate) async fn list_trash_call(
        &self,
        after: Option<&TrashPlace>,
        size: usize,
    ) -> Result<TrashPage> {
        let page = self
            .call(|r| async move { v2_history::list_trash(&r.texts, v2_pages::READ_LIMIT).await })
            .await?;
        let after = after.copied();
        let (items, more) = v2_pages::cut(
            page.items,
            page.more,
            after.map(|a| move |i: &TrashItem| i.place() == a),
            size,
        );
        Ok(TrashPage { items, more })
    }

    pub(crate) async fn restore_from_trash_call(
        &self,
        id: ObjectId,
        path: &str,
    ) -> Result<TextEntry> {
        let path = path.to_string();
        self.call(move |r| async move { v2_history::restore_from_trash(&r.texts, id, path).await })
            .await
    }

    pub(crate) async fn purge_call(&self, id: ObjectId) -> Result<()> {
        self.call(move |r| async move { v2_history::purge(&r.texts, id).await })
            .await
    }

    pub(crate) async fn create_folder_call(
        &self,
        parent: Option<ObjectId>,
        name: &str,
        path: &str,
    ) -> Result<Node> {
        let (name, path) = (name.to_string(), path.to_string());
        self.call(
            move |r| async move { v2_nodes::create_folder(&r.texts, parent, name, path).await },
        )
        .await
    }

    pub(crate) async fn rename_folder_call(&self, id: ObjectId, name: &str) -> Result<Node> {
        let name = name.to_string();
        self.call(move |r| async move { v2_nodes::rename_folder(&r.texts, id, name).await })
            .await
    }

    pub(crate) async fn delete_folder_call(&self, id: ObjectId) -> Result<()> {
        self.call(move |r| async move { v2_nodes::delete_folder(&r.texts, id).await })
            .await
    }

    pub(crate) async fn get_folder_call(&self, id: ObjectId) -> Result<Option<Node>> {
        self.call(move |r| async move { v2_nodes::get_folder(&r.texts, id).await })
            .await
    }

    pub(crate) async fn children_call(
        &self,
        parent: Option<ObjectId>,
        offset: usize,
        size: usize,
    ) -> Result<NodePage> {
        self.call(move |r| async move { v2_nodes::children(&r.texts, parent, offset, size).await })
            .await
    }

    pub(crate) async fn reorder_call(
        &self,
        id: ObjectId,
        placement: Placement,
        anchor: Option<ObjectId>,
    ) -> Result<Node> {
        self.call(move |r| async move { v2_nodes::reorder(&r.texts, id, placement, anchor).await })
            .await
    }

    pub(crate) async fn set_template_call(
        &self,
        id: ObjectId,
        template: bool,
        expect: Expect,
    ) -> Result<TextEntry> {
        self.call(move |r| async move { v2::set_template(&r.texts, id, template, expect).await })
            .await
    }

    pub(crate) async fn create_from_template_call(
        &self,
        template: ObjectId,
        path: &str,
    ) -> Result<TextEntry> {
        let path = path.to_string();
        self.call(move |r| async move { v2::create_from_template(&r.texts, template, path).await })
            .await
    }

    pub(crate) async fn set_redirect_call(
        &self,
        id: ObjectId,
        target: &str,
        expect: Expect,
    ) -> Result<TextEntry> {
        let target = target.to_string();
        self.call(move |r| async move { v2::set_redirect(&r.texts, id, target, expect).await })
            .await
    }

    pub(crate) async fn resolve_call(&self, path: &str) -> Result<Resolution> {
        let path = path.to_string();
        self.call(move |r| async move { v2::resolve(&r.texts, path).await })
            .await
    }

    pub(crate) async fn search_call(
        &self,
        query: &SearchQuery,
        after: Option<&SearchPlace>,
        size: usize,
    ) -> Result<SearchPage> {
        let query = query.clone();
        let page = self
            .call(move |r| async move { v2::search(&r.texts, query, v2_pages::READ_LIMIT).await })
            .await?;
        // The relevance is not on the wire: a client continues a search by the id of the last hit it saw.
        let after = after.map(|a| a.id);
        let (items, more) = v2_pages::cut(
            page.items,
            page.more,
            after.map(|id| move |h: &SearchHit| h.id == id),
            size,
        );
        Ok(SearchPage { items, more })
    }
}
