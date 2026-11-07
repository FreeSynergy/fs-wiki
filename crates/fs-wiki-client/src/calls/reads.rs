//! Calls that only read.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core.

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_store::ObjectId;
use fs_wiki_core::{LinkEntry, Paging, Result, TextEntry};

use super::pages::{collect, read_page};
use crate::problems::{error_of, Subject};
use crate::read::{entry, link, unreadable};

/// `get`: `None` when the server says `found = false`.
pub(crate) async fn get(c: &Client, id: ObjectId) -> Result<Option<TextEntry>> {
    let mut req = c.get_request();
    write_object_id(req.get().init_id(), id);
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    let r = answer.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    entry(r.get_text().map_err(unreadable)?).map(Some)
}

/// `getByPath`.
pub(crate) async fn get_by_path(c: &Client, path: String) -> Result<Option<TextEntry>> {
    let mut req = c.get_by_path_request();
    req.get().set_path(path.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    let r = answer.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    entry(r.get_text().map_err(unreadable)?).map(Some)
}

/// `list`, every page needed for `page`.
pub(crate) async fn list(c: &Client, prefix: String, page: Paging) -> Result<Vec<TextEntry>> {
    collect(page, |cursor| {
        let mut req = c.list_request();
        req.get().set_prefix(prefix.as_str());
        req.get().init_cursor().set_opaque(&cursor);
        async move {
            let answer = req
                .send()
                .promise
                .await
                .map_err(|e| error_of(&e, &Subject::None))?;
            read_page(
                answer
                    .get()
                    .and_then(|r| r.get_page())
                    .map_err(unreadable)?,
            )
        }
    })
    .await
}

/// `find`, every page needed for `page`.
pub(crate) async fn find(c: &Client, term: String, page: Paging) -> Result<Vec<TextEntry>> {
    collect(page, |cursor| {
        let mut req = c.find_request();
        req.get().set_term(term.as_str());
        req.get().init_cursor().set_opaque(&cursor);
        async move {
            let answer = req
                .send()
                .promise
                .await
                .map_err(|e| error_of(&e, &Subject::None))?;
            read_page(
                answer
                    .get()
                    .and_then(|r| r.get_page())
                    .map_err(unreadable)?,
            )
        }
    })
    .await
}

/// `links`.
pub(crate) async fn links(c: &Client, id: ObjectId) -> Result<Vec<LinkEntry>> {
    let mut req = c.links_request();
    write_object_id(req.get().init_id(), id);
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    let rows = answer
        .get()
        .and_then(|r| r.get_items())
        .map_err(unreadable)?;
    rows.iter().map(link).collect()
}

/// `backlinks`.
pub(crate) async fn backlinks(c: &Client, path: String) -> Result<Vec<LinkEntry>> {
    let mut req = c.backlinks_request();
    req.get().set_path(path.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    let rows = answer
        .get()
        .and_then(|r| r.get_items())
        .map_err(unreadable)?;
    rows.iter().map(link).collect()
}

/// `exportText`.
pub(crate) async fn export_text(c: &Client, id: ObjectId, media_type: String) -> Result<String> {
    let mut req = c.export_text_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_media_type(media_type.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    let text = answer
        .get()
        .and_then(|r| r.get_text())
        .map_err(unreadable)?;
    text.to_string().map_err(unreadable)
}
