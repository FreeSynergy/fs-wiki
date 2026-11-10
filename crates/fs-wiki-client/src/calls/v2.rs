//! The calls of texts 0.2: history, trash, folders, templates, redirects, search, and the type catalog.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core (as `reads.rs`/`writes.rs`). The
//! cursors of the contract are opaque, so a list that continues after a place is read from the start and cut there:
//! `drop_through` in `v2_pages.rs` (pages are small, the lists of a wiki are not long).

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_objects::ObjectId;
use fs_types::TypeRegistry;
use fs_wiki_core::{Expect, Resolution, Result, SearchHit, SearchPage, SearchQuery, TextEntry};

use super::v2_pages::collect;
use super::v2_wire::{outcome, write_expect, write_query};
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
