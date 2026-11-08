//! The read methods added by texts 0.2: one call of the provider each, nothing is written.
//!
//! Pattern: Adapter — wire request in, `Texts` call, wire response out. A page is at most the size
//! the contract allows and the answer is checked against the message size; a cursor is the place the page ended at (`cursors.rs`).

use std::sync::Arc;

use fs_interfaces::rpc::{check_answer, read_object_id};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{Texts, MAX_CHILDREN_PAGE, MAX_HISTORY_PAGE, MAX_SEARCH_PAGE};

use crate::cursors;
use crate::errors_v2::rpc_error_v2;
use crate::wire::{count, required, write_entry, Sent};
use crate::wire_history::{info_weight, trash_weight, write_info, write_trash_item};
use crate::wire_nodes::{node_weight, optional_id, write_node};
use crate::wire_search::{hit_weight, outcome, read_query, write_hit};

type Done = Result<(), capnp::Error>;

pub(crate) async fn history(
    texts: &Arc<dyn Texts>,
    size: usize,
    params: texts::HistoryParams,
    mut results: texts::HistoryResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let before = cursors::read_number(p.get_cursor()?.get_opaque()?)?;
    let page = texts
        .history(id, before, size.min(MAX_HISTORY_PAGE))
        .await
        .map_err(rpc_error_v2)?;
    check_answer(page.items.iter().map(info_weight).sum())?;
    let mut out = results.get().init_page();
    let mut rows = out.reborrow().init_items(count(page.items.len())?);
    for (i, info) in (0u32..).zip(&page.items) {
        write_info(rows.reborrow().get(i), info);
    }
    let next = match page.items.last() {
        Some(end) if page.more => cursors::number(end.number),
        _ => Vec::new(),
    };
    out.init_next().set_opaque(&next);
    Ok(())
}

pub(crate) async fn get_revision(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::GetRevisionParams,
    mut results: texts::GetRevisionResults,
) -> Done {
    let p = params.get()?;
    let (id, number) = (read_object_id(p.get_id()?)?, p.get_number());
    let found = texts.get_revision(id, number).await.map_err(rpc_error_v2)?;
    let mut r = results.get();
    r.set_found(found.is_some());
    let Some(content) = found else {
        return Ok(());
    };
    let json = serde_json::to_string(&content.document)
        .map_err(|e| capnp::Error::failed(format!("document does not write: {e}")))?;
    check_answer(json.len() + info_weight(&content.info))?;
    let mut out = r.init_revision();
    write_info(out.reborrow().init_info(), &content.info);
    out.set_document(json.as_str());
    Ok(())
}

pub(crate) async fn list_trash(
    texts: &Arc<dyn Texts>,
    size: usize,
    params: texts::ListTrashParams,
    mut results: texts::ListTrashResults,
) -> Done {
    let after = cursors::read_trash(params.get()?.get_cursor()?.get_opaque()?)?;
    let page = texts
        .list_trash(after.as_ref(), size)
        .await
        .map_err(rpc_error_v2)?;
    check_answer(page.items.iter().map(trash_weight).sum())?;
    let mut out = results.get().init_page();
    let mut rows = out.reborrow().init_items(count(page.items.len())?);
    for (i, item) in (0u32..).zip(&page.items) {
        write_trash_item(rows.reborrow().get(i), item);
    }
    let next = match page.items.last() {
        Some(end) if page.more => cursors::trash(&end.place()),
        _ => Vec::new(),
    };
    out.init_next().set_opaque(&next);
    Ok(())
}

pub(crate) async fn get_folder(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::GetFolderParams,
    mut results: texts::GetFolderResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let node = texts.get_folder(id).await.map_err(rpc_error_v2)?;
    let mut r = results.get();
    r.set_found(node.is_some());
    if let Some(n) = node {
        write_node(r.init_node(), &n);
    }
    Ok(())
}

pub(crate) async fn children(
    texts: &Arc<dyn Texts>,
    size: usize,
    params: texts::ChildrenParams,
    mut results: texts::ChildrenResults,
) -> Done {
    let p = params.get()?;
    let parent = optional_id(p.get_parent()?)?;
    let offset = cursors::read_offset(p.get_cursor()?.get_opaque()?)?.unwrap_or(0);
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let page = texts
        .children(parent, offset, size.min(MAX_CHILDREN_PAGE))
        .await
        .map_err(rpc_error_v2)?;
    check_answer(page.items.iter().map(node_weight).sum())?;
    let mut out = results.get().init_page();
    let mut rows = out.reborrow().init_items(count(page.items.len())?);
    for (i, node) in (0u32..).zip(&page.items) {
        write_node(rows.reborrow().get(i), node);
    }
    let next = if page.more {
        cursors::offset((offset + page.items.len()) as u64)
    } else {
        Vec::new()
    };
    out.init_next().set_opaque(&next);
    Ok(())
}

pub(crate) async fn resolve(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::ResolveParams,
    mut results: texts::ResolveResults,
) -> Done {
    let path = required(params.get()?.get_path()?, "path")?.to_string();
    let found = texts.resolve(&path).await.map_err(rpc_error_v2)?;
    let mut out = results.get().init_resolution();
    out.set_outcome(outcome(found.outcome));
    let mut hops = out.reborrow().init_hops(count(found.hops.len())?);
    for (i, hop) in (0u32..).zip(&found.hops) {
        hops.set(i, hop.as_str());
    }
    if let Some(text) = found.text {
        write_entry(out.init_text(), &Sent::new(text)?)?;
    }
    Ok(())
}

pub(crate) async fn search(
    texts: &Arc<dyn Texts>,
    size: usize,
    params: texts::SearchParams,
    mut results: texts::SearchResults,
) -> Done {
    let p = params.get()?;
    let query = read_query(p.get_query()?)?;
    let after = cursors::read_search(p.get_cursor()?.get_opaque()?)?;
    let page = texts
        .search(&query, after.as_ref(), size.min(MAX_SEARCH_PAGE))
        .await
        .map_err(rpc_error_v2)?;
    check_answer(page.items.iter().map(hit_weight).sum())?;
    let mut out = results.get().init_page();
    let mut rows = out.reborrow().init_items(count(page.items.len())?);
    for (i, hit) in (0u32..).zip(&page.items) {
        write_hit(rows.reborrow().get(i), hit);
    }
    let next = match page.items.last() {
        Some(end) if page.more => cursors::search(&end.place()),
        _ => Vec::new(),
    };
    out.init_next().set_opaque(&next);
    Ok(())
}
