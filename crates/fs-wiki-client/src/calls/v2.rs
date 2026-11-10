//! The calls of texts 0.2: history, trash, folders, templates, redirects, search, and the type catalog.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core (as `reads.rs`/`writes.rs`). The
//! cursors of the contract are opaque, so a list that continues after a place is read from the start and cut there:
//! `drop_through` in `v2_pages.rs` (pages are small, the lists of a wiki are not long).

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_objects::ObjectId;
use fs_text::Document;
use fs_types::TypeRegistry;
use fs_wiki_core::{
    Expect, Node, NodePage, Placement, Resolution, Result, RevisionContent, RevisionPage,
    SearchHit, SearchPage, SearchQuery, TextEntry, TrashItem, TrashPage,
};

use super::v2_pages::collect;
use super::v2_wire::{info, node, outcome, placement, trash_item, write_expect, write_query};
use crate::problems::{error_of, Subject};
use crate::read::{entry, unreadable};

/// Send a request and wait for the reply; the error of the core for what the server said.
macro_rules! answer {
    ($req:expr, $subject:expr) => {
        $req.send()
            .promise
            .await
            .map_err(|e| error_of(&e, &$subject))?
    };
}

fn next_of(c: capnp::Result<fs_interfaces::common_capnp::cursor::Reader<'_>>) -> Result<Vec<u8>> {
    Ok(c.and_then(|c| c.get_opaque()).map_err(unreadable)?.to_vec())
}

/// `Texts.types`: the catalog of the wiki's data types, read and checked.
pub(crate) async fn types(c: &Client) -> Result<TypeRegistry> {
    let catalog = answer!(c.types_request(), Subject::None)
        .get()
        .and_then(|r| r.get_catalog())
        .map_err(unreadable)?;
    let reply = catalog
        .describe_request()
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    TypeRegistry::read(
        reply
            .get()
            .and_then(|r| r.get_types())
            .map_err(unreadable)?,
    )
    .map_err(unreadable)
}

pub(crate) async fn edit_with(
    c: &Client,
    id: ObjectId,
    expect: Expect,
    document: String,
    summary: String,
) -> Result<TextEntry> {
    let mut req = c.edit_with_request();
    write_object_id(req.get().init_id(), id);
    write_expect(req.get().init_expect(), expect);
    req.get().set_document(document.as_str());
    req.get().set_summary(summary.as_str());
    let subject = match expect {
        Expect::Revision(n) => Subject::Revision(id, n),
        Expect::Any => Subject::Text(id),
    };
    entry(
        answer!(req, subject)
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn history(c: &Client, id: ObjectId, size: usize) -> Result<RevisionPage> {
    let (items, more) = collect(size, |cursor| async move {
        let mut req = c.history_request();
        write_object_id(req.get().init_id(), id);
        req.get().init_cursor().set_opaque(&cursor);
        let reply = answer!(req, Subject::Text(id));
        let page = reply.get().and_then(|r| r.get_page()).map_err(unreadable)?;
        let items = page
            .get_items()
            .map_err(unreadable)?
            .iter()
            .map(info)
            .collect::<Result<Vec<_>>>()?;
        Ok((items, next_of(page.get_next())?))
    })
    .await?;
    Ok(RevisionPage { items, more })
}

pub(crate) async fn get_revision(
    c: &Client,
    id: ObjectId,
    number: u64,
) -> Result<Option<RevisionContent>> {
    let mut req = c.get_revision_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_number(number);
    let reply = answer!(req, Subject::Text(id));
    let r = reply.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    let r = r.get_revision().map_err(unreadable)?;
    let document: Document = serde_json::from_str(
        r.get_document()
            .map_err(unreadable)?
            .to_str()
            .map_err(unreadable)?,
    )
    .map_err(unreadable)?;
    Ok(Some(RevisionContent {
        info: info(r.get_info().map_err(unreadable)?)?,
        document,
    }))
}

pub(crate) async fn restore_revision(
    c: &Client,
    id: ObjectId,
    number: u64,
    expect: Expect,
) -> Result<TextEntry> {
    let mut req = c.restore_revision_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_number(number);
    write_expect(req.get().init_expect(), expect);
    entry(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn purge_revision(c: &Client, id: ObjectId, number: u64) -> Result<()> {
    let mut req = c.purge_revision_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_number(number);
    answer!(req, Subject::Text(id));
    Ok(())
}

pub(crate) async fn trash(c: &Client, id: ObjectId) -> Result<TrashItem> {
    let mut req = c.trash_request();
    write_object_id(req.get().init_id(), id);
    trash_item(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_item())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn list_trash(c: &Client, size: usize) -> Result<TrashPage> {
    let (items, more) = collect(size, |cursor| async move {
        let mut req = c.list_trash_request();
        req.get().init_cursor().set_opaque(&cursor);
        let reply = answer!(req, Subject::None);
        let page = reply.get().and_then(|r| r.get_page()).map_err(unreadable)?;
        let items = page
            .get_items()
            .map_err(unreadable)?
            .iter()
            .map(trash_item)
            .collect::<Result<Vec<_>>>()?;
        Ok((items, next_of(page.get_next())?))
    })
    .await?;
    Ok(TrashPage { items, more })
}

pub(crate) async fn restore_from_trash(
    c: &Client,
    id: ObjectId,
    path: String,
) -> Result<TextEntry> {
    let mut req = c.restore_from_trash_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_path(path.as_str());
    entry(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn purge(c: &Client, id: ObjectId) -> Result<()> {
    let mut req = c.purge_request();
    write_object_id(req.get().init_id(), id);
    answer!(req, Subject::Text(id));
    Ok(())
}

pub(crate) async fn create_folder(
    c: &Client,
    parent: Option<ObjectId>,
    name: String,
    path: String,
) -> Result<Node> {
    let mut req = c.create_folder_request();
    fs_interfaces::rpc::write_uuid(req.get().init_parent(), parent.map(|p| p.as_uuid()));
    req.get().set_name(name.as_str());
    req.get().set_path(path.as_str());
    node(
        answer!(req, Subject::None)
            .get()
            .and_then(|r| r.get_node())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn rename_folder(c: &Client, id: ObjectId, name: String) -> Result<Node> {
    let mut req = c.rename_folder_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_name(name.as_str());
    node(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_node())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn delete_folder(c: &Client, id: ObjectId) -> Result<()> {
    let mut req = c.delete_folder_request();
    write_object_id(req.get().init_id(), id);
    answer!(req, Subject::Text(id));
    Ok(())
}

pub(crate) async fn get_folder(c: &Client, id: ObjectId) -> Result<Option<Node>> {
    let mut req = c.get_folder_request();
    write_object_id(req.get().init_id(), id);
    let reply = answer!(req, Subject::Text(id));
    let r = reply.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    node(r.get_node().map_err(unreadable)?).map(Some)
}

pub(crate) async fn children(
    c: &Client,
    parent: Option<ObjectId>,
    offset: usize,
    size: usize,
) -> Result<NodePage> {
    let (items, more) = collect(offset.saturating_add(size), |cursor| async move {
        let mut req = c.children_request();
        fs_interfaces::rpc::write_uuid(req.get().init_parent(), parent.map(|p| p.as_uuid()));
        req.get().init_cursor().set_opaque(&cursor);
        let reply = answer!(req, Subject::None);
        let page = reply.get().and_then(|r| r.get_page()).map_err(unreadable)?;
        let items = page
            .get_items()
            .map_err(unreadable)?
            .iter()
            .map(node)
            .collect::<Result<Vec<_>>>()?;
        Ok((items, next_of(page.get_next())?))
    })
    .await?;
    Ok(NodePage {
        items: items.into_iter().skip(offset).collect(),
        more,
    })
}

pub(crate) async fn reorder(
    c: &Client,
    id: ObjectId,
    at: Placement,
    anchor: Option<ObjectId>,
) -> Result<Node> {
    let mut req = c.reorder_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_placement(placement(at));
    fs_interfaces::rpc::write_uuid(req.get().init_anchor(), anchor.map(|p| p.as_uuid()));
    node(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_node())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn set_template(
    c: &Client,
    id: ObjectId,
    template: bool,
    expect: Expect,
) -> Result<TextEntry> {
    let mut req = c.set_template_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_template(template);
    write_expect(req.get().init_expect(), expect);
    entry(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn create_from_template(
    c: &Client,
    template: ObjectId,
    path: String,
) -> Result<TextEntry> {
    let mut req = c.create_from_template_request();
    write_object_id(req.get().init_template(), template);
    req.get().set_path(path.as_str());
    entry(
        answer!(req, Subject::Text(template))
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn set_redirect(
    c: &Client,
    id: ObjectId,
    target: String,
    expect: Expect,
) -> Result<TextEntry> {
    let mut req = c.set_redirect_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_target(target.as_str());
    write_expect(req.get().init_expect(), expect);
    entry(
        answer!(req, Subject::Text(id))
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

pub(crate) async fn resolve(c: &Client, path: String) -> Result<Resolution> {
    let mut req = c.resolve_request();
    req.get().set_path(path.as_str());
    let reply = answer!(req, Subject::None);
    let r = reply
        .get()
        .and_then(|r| r.get_resolution())
        .map_err(unreadable)?;
    let hops = r
        .get_hops()
        .map_err(unreadable)?
        .iter()
        .map(|h| {
            h.map_err(unreadable)
                .and_then(|h| h.to_string().map_err(unreadable))
        })
        .collect::<Result<Vec<_>>>()?;
    let kind = outcome(r.get_outcome())?;
    let text = if r.has_text() {
        Some(entry(r.get_text().map_err(unreadable)?)?)
    } else {
        None
    };
    Ok(Resolution {
        outcome: kind,
        hops,
        text,
    })
}

pub(crate) async fn search(c: &Client, query: SearchQuery, size: usize) -> Result<SearchPage> {
    let (items, more) = collect(size, |cursor| {
        let query = query.clone();
        async move {
            let mut req = c.search_request();
            write_query(req.get().init_query(), &query);
            req.get().init_cursor().set_opaque(&cursor);
            let reply = answer!(req, Subject::None);
            let page = reply.get().and_then(|r| r.get_page()).map_err(unreadable)?;
            let items = page
                .get_items()
                .map_err(unreadable)?
                .iter()
                .map(|h| super::v2_wire::hit(h, 0))
                .collect::<Result<Vec<SearchHit>>>()?;
            Ok((items, next_of(page.get_next())?))
        }
    })
    .await?;
    Ok(SearchPage { items, more })
}
