//! The [`Texts`] contract: what the wiki program offers to everyone else.
//!
//! Pattern: Repository (interface) + Null Object — callers depend on this trait, never on the
//! store-backed `WikiRepo`. It is the Rust form of `Texts` in `texts.capnp` (fs-interfaces). Like
//! a parent class (ADR 0045) every provider has every method: one it does not override answers
//! an explicit [`Unsupported`], and `capabilities` reports the same.

use async_trait::async_trait;
use fs_interfaces::{Capabilities, Unsupported};
use fs_objects::{ObjectId, StoreId};
use fs_text::Document;

pub use crate::entry::{LinkEntry, Paging, TextEntry};
use crate::error::Result;
use crate::history::{Expect, RevisionContent, RevisionPage};
use crate::nodes::{Node, NodePage, Placement};
use crate::page::{Place, TextPage, TextQuery};
use crate::redirect::Resolution;
use crate::search::{SearchPage, SearchPlace, SearchQuery};
use crate::trash::{TrashItem, TrashPage, TrashPlace};

/// The explicit "not possible" of a method a provider does not override.
pub(crate) fn not_possible<T>(capability: &str) -> Result<T> {
    Err(Unsupported::new(capability, "this provider does not offer it").into())
}

/// Texts: pages with layers, found by path or term, linked to each other.
///
/// Interface status: draft (ADR 0045).
#[async_trait]
pub trait Texts: Send + Sync {
    /// What this provider can do, per capability id.
    fn capabilities(&self) -> Capabilities;
    /// The identity of the store behind this provider, if it has one: made once when the store is
    /// created, never in the log. A provider without a store (a fake) answers `None`.
    fn identity(&self) -> Option<StoreId> {
        None
    }
    /// `text.read`: the text, if it exists.
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>>;
    /// `text.read`: lookup by readable path.
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>>;
    /// `text.read`: texts whose path starts with `prefix` (empty = all), in path order, paged.
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>>;
    /// `text.read` / `text.find`: at most `size` texts of `query` that come after `after` (`None` =
    /// from the start), in path order. A provider with a store cuts the page in the store: it reads
    /// neither the texts before the place nor the ones after the page. A `Term` query is `text.find`:
    /// a provider without it answers [`Unsupported`].
    async fn page(&self, query: &TextQuery, after: Option<&Place>, size: usize)
        -> Result<TextPage>;

    /// `text.create`: a new text at `path` from Markdown. A taken path is an error.
    async fn create(&self, _path: &str, _markdown: &str) -> Result<TextEntry> {
        not_possible("text.create")
    }
    /// `text.edit`: replace the document of `id` (the id of the document is forced to `id`).
    async fn edit(&self, _id: ObjectId, _document: Document) -> Result<TextEntry> {
        not_possible("text.edit")
    }
    /// `text.edit` only if the text still has `revision` (compare-and-set, `TextEntry::revision`);
    /// otherwise `Error::Conflict` and nothing is written. Atomic inside one process only.
    async fn edit_if(
        &self,
        _id: ObjectId,
        _revision: u64,
        _document: Document,
    ) -> Result<TextEntry> {
        not_possible("text.edit")
    }
    /// `text.move`: give `id` a new path; links to the old path in other texts follow.
    async fn move_to(&self, _id: ObjectId, _path: &str) -> Result<TextEntry> {
        not_possible("text.move")
    }
    /// `text.move` for a folder: the text at `prefix` (if there is one) and every text below
    /// `prefix/` move to `path` and `path/...`; links in all texts follow. Returns the moved texts
    /// in their new places, shallow ones first. A clash with an existing text refuses the whole
    /// move before anything is written; repeating an interrupted call finishes it.
    async fn move_folder(&self, _prefix: &str, _path: &str) -> Result<Vec<TextEntry>> {
        not_possible("text.move")
    }
    /// `text.delete`: delete a text with its layers (undoable through the store).
    async fn delete(&self, _id: ObjectId) -> Result<()> {
        not_possible("text.delete")
    }
    /// `text.find`: texts whose name, path or text contains `term`, ignoring case.
    async fn find(&self, _term: &str, _page: Paging) -> Result<Vec<TextEntry>> {
        not_possible("text.find")
    }
    /// `link.read`: the links inside `id`.
    async fn links(&self, _id: ObjectId) -> Result<Vec<LinkEntry>> {
        not_possible("link.read")
    }
    /// `link.read`: the links in all texts that point at `path`.
    async fn backlinks(&self, _path: &str) -> Result<Vec<LinkEntry>> {
        not_possible("link.read")
    }
    /// `text.import`: `text` of `media_type` is the file of `path`. A new path creates a text; an
    /// existing one is an outside change: the difference becomes a normal edit and layers stay.
    async fn import_text(&self, _path: &str, _media_type: &str, _text: &str) -> Result<TextEntry> {
        not_possible("text.import")
    }
    /// `text.export`: the text as a file of `media_type` (layers are not written).
    async fn export_text(&self, _id: ObjectId, _media_type: &str) -> Result<String> {
        not_possible("text.export")
    }
    // ---- texts 0.2 (all append only; every one defaults to `Unsupported`) ----------------------------

    /// `text.editSummary`: `edit` / `edit_if` (per `expect`) with a revision summary (at most 200 bytes).
    async fn edit_with(
        &self,
        _id: ObjectId,
        _expect: Expect,
        _document: Document,
        _summary: &str,
    ) -> Result<TextEntry> {
        not_possible("text.editSummary")
    }
    /// `text.history`: revisions of a text (also a trashed one), newest first; `before` = only numbers
    /// below it.
    async fn history(
        &self,
        _id: ObjectId,
        _before: Option<u64>,
        _size: usize,
    ) -> Result<RevisionPage> {
        not_possible("text.history")
    }
    /// `text.history`: one revision with its document; `None` if it never existed or was removed.
    async fn get_revision(&self, _id: ObjectId, _number: u64) -> Result<Option<RevisionContent>> {
        not_possible("text.history")
    }
    /// `text.history`: write an old document as a new revision; path, template and redirect stay.
    async fn restore_revision(
        &self,
        _id: ObjectId,
        _number: u64,
        _expect: Expect,
    ) -> Result<TextEntry> {
        not_possible("text.history")
    }
    /// `text.history`: remove one old revision for good (not the current one).
    async fn purge_revision(&self, _id: ObjectId, _number: u64) -> Result<()> {
        not_possible("text.history")
    }
    /// `text.trash`: move a text with its history out of sight; its path is free again.
    async fn trash(&self, _id: ObjectId) -> Result<TrashItem> {
        not_possible("text.trash")
    }
    /// `text.trash`: trashed texts, most recently deleted first, after `after`.
    async fn list_trash(&self, _after: Option<&TrashPlace>, _size: usize) -> Result<TrashPage> {
        not_possible("text.trash")
    }
    /// `text.trash`: bring a trashed text back at `path` (empty = the old one).
    async fn restore_from_trash(&self, _id: ObjectId, _path: &str) -> Result<TextEntry> {
        not_possible("text.trash")
    }
    /// `text.trash`: remove a trashed text from this provider for good.
    async fn purge(&self, _id: ObjectId) -> Result<()> {
        not_possible("text.trash")
    }
    /// `text.space`: a new space (no parent) or folder.
    async fn create_folder(
        &self,
        _parent: Option<ObjectId>,
        _name: &str,
        _path: &str,
    ) -> Result<Node> {
        not_possible("text.space")
    }
    /// `text.space`: change the display name of a space or folder.
    async fn rename_folder(&self, _id: ObjectId, _name: &str) -> Result<Node> {
        not_possible("text.space")
    }
    /// `text.space`: delete an empty space or folder.
    async fn delete_folder(&self, _id: ObjectId) -> Result<()> {
        not_possible("text.space")
    }
    /// `text.space`: one space or folder.
    async fn get_folder(&self, _id: ObjectId) -> Result<Option<Node>> {
        not_possible("text.space")
    }
    /// `text.space`: the children of `parent` in order, skipping `offset`.
    async fn children(
        &self,
        _parent: Option<ObjectId>,
        _offset: usize,
        _size: usize,
    ) -> Result<NodePage> {
        not_possible("text.space")
    }
    /// `text.space`: move a node among its siblings.
    async fn reorder(
        &self,
        _id: ObjectId,
        _placement: Placement,
        _anchor: Option<ObjectId>,
    ) -> Result<Node> {
        not_possible("text.space")
    }
    /// `text.template`: mark or unmark a text as a template.
    async fn set_template(
        &self,
        _id: ObjectId,
        _template: bool,
        _expect: Expect,
    ) -> Result<TextEntry> {
        not_possible("text.template")
    }
    /// `text.template`: a new text with a copy of the template's document.
    async fn create_from_template(&self, _template: ObjectId, _path: &str) -> Result<TextEntry> {
        not_possible("text.template")
    }
    /// `text.redirect`: make a text forward to `target` (empty clears it).
    async fn set_redirect(
        &self,
        _id: ObjectId,
        _target: &str,
        _expect: Expect,
    ) -> Result<TextEntry> {
        not_possible("text.redirect")
    }
    /// `text.redirect`: follow redirects from `path`, at most 8.
    async fn resolve(&self, _path: &str) -> Result<Resolution> {
        not_possible("text.redirect")
    }
    /// `text.search`: words, snippets and filters; hits come after `after` in the order of the result.
    async fn search(
        &self,
        _query: &SearchQuery,
        _after: Option<&SearchPlace>,
        _size: usize,
    ) -> Result<SearchPage> {
        not_possible("text.search")
    }
}
