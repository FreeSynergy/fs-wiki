//! The read methods of `texts.capnp`: one call of the provider each, nothing is written.
//!
//! Pattern: Adapter — wire request in, `Texts` call, wire response out; an unknown media type of
//! `exportText` is that method's explicit "not possible".

use std::sync::Arc;

use fs_interfaces::rpc::{check_answer, read_object_id};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{Paging, Texts};

use crate::errors::{rpc_error, rpc_error_for_format};
use crate::links::{internal, write_links};
use crate::wire::{count, required, text_arg, write_entry, write_page, Sent};

type Done = Result<(), capnp::Error>;

pub(crate) async fn get(
    texts: &Arc<dyn Texts>,
    params: texts::GetParams,
    mut results: texts::GetResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let entry = texts.get(id).await.map_err(rpc_error)?;
    let mut r = results.get();
    r.set_found(entry.is_some());
    entry.map_or(Ok(()), |e| write_entry(r.init_text(), &Sent::new(e)?))
}

pub(crate) async fn get_by_path(
    texts: &Arc<dyn Texts>,
    params: texts::GetByPathParams,
    mut results: texts::GetByPathResults,
) -> Done {
    let path = required(params.get()?.get_path()?, "path")?.to_string();
    let entry = texts.get_by_path(&path).await.map_err(rpc_error)?;
    let mut r = results.get();
    r.set_found(entry.is_some());
    entry.map_or(Ok(()), |e| write_entry(r.init_text(), &Sent::new(e)?))
}

pub(crate) async fn list(
    texts: &Arc<dyn Texts>,
    params: texts::ListParams,
    mut results: texts::ListResults,
) -> Done {
    let p = params.get()?;
    let prefix = text_arg(p.get_prefix()?)?;
    let all = texts
        .list(prefix, Paging::default())
        .await
        .map_err(rpc_error)?;
    write_page(results.get().init_page(), all, p.get_cursor())
}

pub(crate) async fn find(
    texts: &Arc<dyn Texts>,
    params: texts::FindParams,
    mut results: texts::FindResults,
) -> Done {
    let p = params.get()?;
    let term = required(p.get_term()?, "term")?;
    let all = texts
        .find(term, Paging::default())
        .await
        .map_err(rpc_error)?;
    write_page(results.get().init_page(), all, p.get_cursor())
}

pub(crate) async fn links(
    texts: &Arc<dyn Texts>,
    params: texts::LinksParams,
    mut results: texts::LinksResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let links = texts.links(id).await.map_err(rpc_error)?;
    let rows = internal(&links)?;
    write_links(results.get().init_items(count(rows.len())?), &rows);
    Ok(())
}

pub(crate) async fn backlinks(
    texts: &Arc<dyn Texts>,
    params: texts::BacklinksParams,
    mut results: texts::BacklinksResults,
) -> Done {
    let path = required(params.get()?.get_path()?, "path")?;
    let links = texts.backlinks(path).await.map_err(rpc_error)?;
    let rows = internal(&links)?;
    write_links(results.get().init_items(count(rows.len())?), &rows);
    Ok(())
}

pub(crate) async fn export_text(
    texts: &Arc<dyn Texts>,
    params: texts::ExportTextParams,
    mut results: texts::ExportTextResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let media = text_arg(p.get_media_type()?)?;
    let text = texts
        .export_text(id, media)
        .await
        .map_err(|e| rpc_error_for_format("text.export", e))?;
    check_answer(text.len())?;
    results.get().set_text(text.as_str());
    Ok(())
}
