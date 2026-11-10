//! The calls of texts 0.2 for history and trash.
//!
//! Pattern: Adapter, as `v2.rs` (split for the file limit): one request in, one reply read back as values of the core.

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_objects::ObjectId;
use fs_text::Document;
use fs_wiki_core::{
    Expect, Result, RevisionContent, RevisionPage, TextEntry, TrashItem, TrashPage,
};

use super::v2_pages::collect;
use super::v2_wire::{info, trash_item, write_expect};
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
