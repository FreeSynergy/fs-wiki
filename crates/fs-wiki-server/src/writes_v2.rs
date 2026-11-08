//! The write methods added by texts 0.2: one call of the provider each, run to its end.
//!
//! Pattern: Adapter + Command (detached execution) — as `writes.rs`; the problems are the typed ones
//! of `errors_v2.rs`. A request is read and bounded first.

use std::future::Future;
use std::sync::Arc;

use fs_interfaces::rpc::{read_object_id, run_to_end};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{Error, TextEntry, Texts};

use crate::errors_v2::rpc_error_v2;
use crate::wire::{read_document, required, text_arg, write_entry, Sent};
use crate::wire_history::{read_expect, write_trash_item};
use crate::wire_nodes::{optional_id, read_placement, write_node};

type Done = Result<(), capnp::Error>;

/// A write that answers a text, run to its end.
async fn written(
    work: impl Future<Output = Result<TextEntry, Error>> + 'static,
) -> capnp::Result<Sent> {
    Sent::new(run_to_end(work).await?.map_err(rpc_error_v2)?)
}

/// A write that answers nothing or something small, run to its end.
async fn run<T: 'static>(
    work: impl Future<Output = Result<T, Error>> + 'static,
) -> capnp::Result<T> {
    run_to_end(work).await?.map_err(rpc_error_v2)
}

pub(crate) async fn edit_with(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::EditWithParams,
    mut results: texts::EditWithResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let expect = read_expect(p.get_expect()?)?;
    let document = read_document(p.get_document()?)?;
    let summary = text_arg(p.get_summary()?)?.to_string();
    let texts = texts.clone();
    let sent =
        written(async move { texts.edit_with(id, expect, document, &summary).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn restore_revision(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::RestoreRevisionParams,
    mut results: texts::RestoreRevisionResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let number = p.get_number();
    let expect = read_expect(p.get_expect()?)?;
    let texts = texts.clone();
    let sent = written(async move { texts.restore_revision(id, number, expect).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn purge_revision(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::PurgeRevisionParams,
    _: texts::PurgeRevisionResults,
) -> Done {
    let p = params.get()?;
    let (id, number) = (read_object_id(p.get_id()?)?, p.get_number());
    let texts = texts.clone();
    run(async move { texts.purge_revision(id, number).await }).await
}

pub(crate) async fn trash(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::TrashParams,
    mut results: texts::TrashResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let texts = texts.clone();
    let item = run(async move { texts.trash(id).await }).await?;
    write_trash_item(results.get().init_item(), &item);
    Ok(())
}

pub(crate) async fn restore_from_trash(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::RestoreFromTrashParams,
    mut results: texts::RestoreFromTrashResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let path = text_arg(p.get_path()?)?.to_string();
    let texts = texts.clone();
    let sent = written(async move { texts.restore_from_trash(id, &path).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn purge(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::PurgeParams,
    _: texts::PurgeResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let texts = texts.clone();
    run(async move { texts.purge(id).await }).await
}

pub(crate) async fn create_folder(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::CreateFolderParams,
    mut results: texts::CreateFolderResults,
) -> Done {
    let p = params.get()?;
    let parent = optional_id(p.get_parent()?)?;
    let name = required(p.get_name()?, "name")?.to_string();
    let path = text_arg(p.get_path()?)?.to_string();
    let texts = texts.clone();
    let node = run(async move { texts.create_folder(parent, &name, &path).await }).await?;
    write_node(results.get().init_node(), &node);
    Ok(())
}

pub(crate) async fn rename_folder(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::RenameFolderParams,
    mut results: texts::RenameFolderResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let name = required(p.get_name()?, "name")?.to_string();
    let texts = texts.clone();
    let node = run(async move { texts.rename_folder(id, &name).await }).await?;
    write_node(results.get().init_node(), &node);
    Ok(())
}

pub(crate) async fn delete_folder(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::DeleteFolderParams,
    _: texts::DeleteFolderResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let texts = texts.clone();
    run(async move { texts.delete_folder(id).await }).await
}

pub(crate) async fn reorder(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::ReorderParams,
    mut results: texts::ReorderResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let placement = read_placement(p.get_placement())?;
    let anchor = optional_id(p.get_anchor()?)?;
    let texts = texts.clone();
    let node = run(async move { texts.reorder(id, placement, anchor).await }).await?;
    write_node(results.get().init_node(), &node);
    Ok(())
}

pub(crate) async fn set_template(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::SetTemplateParams,
    mut results: texts::SetTemplateResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let (template, expect) = (p.get_template(), read_expect(p.get_expect()?)?);
    let texts = texts.clone();
    let sent = written(async move { texts.set_template(id, template, expect).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn create_from_template(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::CreateFromTemplateParams,
    mut results: texts::CreateFromTemplateResults,
) -> Done {
    let p = params.get()?;
    let template = read_object_id(p.get_template()?)?;
    let path = required(p.get_path()?, "path")?.to_string();
    let texts = texts.clone();
    let sent = written(async move { texts.create_from_template(template, &path).await }).await?;
    write_entry(results.get().init_text(), &sent)
}

pub(crate) async fn set_redirect(
    texts: &Arc<dyn Texts>,
    _: usize,
    params: texts::SetRedirectParams,
    mut results: texts::SetRedirectResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let target = text_arg(p.get_target()?)?.to_string();
    let expect = read_expect(p.get_expect()?)?;
    let texts = texts.clone();
    let sent = written(async move { texts.set_redirect(id, &target, expect).await }).await?;
    write_entry(results.get().init_text(), &sent)
}
