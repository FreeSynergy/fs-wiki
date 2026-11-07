//! The write methods of `texts.capnp`: one call of the provider each, run to its end.
//!
//! Pattern: Adapter + Command (detached execution) — every write goes through `run_to_end`, so a
//! client that goes away cannot leave a move half done. A request is read and bounded first, and a
//! document that could not come back in an answer is refused before anything is stored.

use std::future::Future;
use std::sync::Arc;

use fs_interfaces::rpc::{read_object_id, run_to_end};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{Error, TextEntry, TextFormat, Texts};

use crate::errors::{rpc_error, rpc_error_for_format};
use crate::wire::{
    count, fits, read_document, required, source_arg, text_arg, write_all, write_entry, Sent,
};

type Done = Result<(), capnp::Error>;

/// A write, run to its end whatever the client does, and its text ready to be answered.
async fn written(
    work: impl Future<Output = Result<TextEntry, Error>> + 'static,
) -> capnp::Result<Sent> {
    Sent::new(run_to_end(work).await?.map_err(rpc_error)?)
}

pub(crate) async fn create(
    texts: &Arc<dyn Texts>,
    params: texts::CreateParams,
    mut results: texts::CreateResults,
) -> Done {
    let p = params.get()?;
    let path = required(p.get_path()?, "path")?.to_string();
    let markdown = source_arg(p.get_markdown()?)?.to_string();
    fits(&TextFormat::Markdown.import(&markdown))?;
    let texts = texts.clone();
    let sent = written(async move { texts.create(&path, &markdown).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn edit(
    texts: &Arc<dyn Texts>,
    params: texts::EditParams,
    mut results: texts::EditResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let document = read_document(p.get_document()?)?;
    let texts = texts.clone();
    let sent = written(async move { texts.edit(id, document).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn edit_if(
    texts: &Arc<dyn Texts>,
    params: texts::EditIfParams,
    mut results: texts::EditIfResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let revision = p.get_revision();
    let document = read_document(p.get_document()?)?;
    let texts = texts.clone();
    let sent = written(async move { texts.edit_if(id, revision, document).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn move_(
    texts: &Arc<dyn Texts>,
    params: texts::MoveParams,
    mut results: texts::MoveResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let path = required(p.get_path()?, "path")?.to_string();
    let texts = texts.clone();
    let sent = written(async move { texts.move_to(id, &path).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn move_folder(
    texts: &Arc<dyn Texts>,
    params: texts::MoveFolderParams,
    mut results: texts::MoveFolderResults,
) -> Done {
    let p = params.get()?;
    let prefix = required(p.get_prefix()?, "prefix")?.to_string();
    let path = required(p.get_path()?, "path")?.to_string();
    let texts = texts.clone();
    let moved = run_to_end(async move { texts.move_folder(&prefix, &path).await })
        .await?
        .map_err(rpc_error)?;
    write_all(results.get().init_moved(count(moved.len())?), moved)
}

pub(crate) async fn delete(
    texts: &Arc<dyn Texts>,
    params: texts::DeleteParams,
    _: texts::DeleteResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let texts = texts.clone();
    run_to_end(async move { texts.delete(id).await })
        .await?
        .map_err(rpc_error)
}

pub(crate) async fn import_text(
    texts: &Arc<dyn Texts>,
    params: texts::ImportTextParams,
    mut results: texts::ImportTextResults,
) -> Done {
    let p = params.get()?;
    let path = required(p.get_path()?, "path")?.to_string();
    let media = text_arg(p.get_media_type()?)?.to_string();
    let text = source_arg(p.get_text()?)?.to_string();
    if let Ok(format) = TextFormat::from_media_type(&media) {
        fits(&format.import(&text))?;
    }
    let texts = texts.clone();
    let entry = run_to_end(async move { texts.import_text(&path, &media, &text).await })
        .await?
        .map_err(|e| rpc_error_for_format("text.import", e))?;
    write_entry(results.get().init_entry(), &Sent::new(entry)?)
}
