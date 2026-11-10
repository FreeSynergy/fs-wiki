//! The calls of texts 0.2 for spaces and folders.
//!
//! Pattern: Adapter, as `v2.rs` (split for the file limit): one request in, one reply read back as values of the core.

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_objects::ObjectId;
use fs_wiki_core::{Node, NodePage, Placement, Result};

use super::v2_pages::collect;
use super::v2_wire::{node, placement};
use crate::problems::{error_of, Subject};
use crate::read::unreadable;

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
