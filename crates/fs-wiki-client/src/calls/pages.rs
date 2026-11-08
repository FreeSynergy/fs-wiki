//! Reading a list page by page.
//!
//! Pattern: Iterator — paging by hand (the pages are fetched by async calls). The contract pages with an opaque cursor, the core with `limit` and `offset`. Pages are read until the cursor is empty or
//! until enough is there for `offset + limit`; the texts before `offset` are read as well, as the
//! cursor is a position the client cannot make.

use std::future::Future;

use fs_interfaces::rpc::{encode_place, PlaceParts};
use fs_interfaces::texts_capnp::text_page;
use fs_objects::sort_key;
use fs_wiki_core::{Paging, Place, Placed, Result, TextEntry, TextPage};

use crate::read::{entry, unreadable};

/// One page as the server sent it: its texts and the cursor behind it (empty = the end).
pub(crate) fn read_page(page: text_page::Reader<'_>) -> Result<(Vec<TextEntry>, Vec<u8>)> {
    let items = page
        .get_items()
        .map_err(unreadable)?
        .iter()
        .map(entry)
        .collect::<Result<_>>()?;
    let next = page
        .get_next()
        .and_then(|n| n.get_opaque())
        .map_err(unreadable)?
        .to_vec();
    Ok((items, next))
}

/// The texts of `page`, fetched with `fetch(cursor)` page after page.
pub(crate) async fn collect<F, Fut>(page: Paging, mut fetch: F) -> Result<Vec<TextEntry>>
where
    F: FnMut(Vec<u8>) -> Fut,
    Fut: Future<Output = Result<(Vec<TextEntry>, Vec<u8>)>>,
{
    if page.limit == Some(0) {
        return Ok(Vec::new());
    }
    let enough = page.limit.map(|limit| page.offset.saturating_add(limit));
    let mut found: Vec<TextEntry> = Vec::new();
    let mut cursor = Vec::new();
    loop {
        let (items, next) = fetch(cursor).await?;
        // A page with nothing on it and a cursor would repeat for ever: it ends the list.
        let empty = items.is_empty();
        found.extend(items);
        cursor = next;
        if cursor.is_empty() || empty || enough.is_some_and(|n| found.len() >= n) {
            break;
        }
    }
    let rest = found.into_iter().skip(page.offset);
    Ok(match page.limit {
        Some(limit) => rest.take(limit).collect(),
        None => rest.collect(),
    })
}

/// The page of `size` texts after `after`, fetched with `fetch(cursor)`. The cursor of the contract
/// is a place (`fs_interfaces::rpc::encode_place`, since v0.49.1), so the server cuts at exactly the
/// place the core does (key, name folded, id) and nothing before it is read. A server page is
/// smaller than `size` at times, so pages are read until `size` texts are there.
pub(crate) async fn collect_after<F, Fut>(
    after: Option<&Place>,
    size: usize,
    mut fetch: F,
) -> Result<TextPage>
where
    F: FnMut(Vec<u8>) -> Fut,
    Fut: Future<Output = Result<(Vec<TextEntry>, Vec<u8>)>>,
{
    let mut cursor = after.map_or_else(Vec::new, |p| {
        encode_place(&PlaceParts {
            key: p.key.clone(),
            name: p.name.clone(),
            id: p.id,
        })
    });
    let mut items: Vec<Placed> = Vec::new();
    loop {
        let (page, next) = fetch(cursor).await?;
        let empty = page.is_empty();
        for entry in page {
            if items.len() == size {
                return Ok(TextPage { items, more: true });
            }
            let place = Place {
                key: sort_key(&entry.path),
                name: entry.envelope.name.clone(),
                id: entry.id,
            };
            items.push(Placed { entry, place });
        }
        cursor = next;
        // A page with nothing on it and a cursor would repeat for ever: it ends the list.
        if cursor.is_empty() || empty {
            return Ok(TextPage { items, more: false });
        }
        if items.len() == size {
            return Ok(TextPage { items, more: true });
        }
    }
}
