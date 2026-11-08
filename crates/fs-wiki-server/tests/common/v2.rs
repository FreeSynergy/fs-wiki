//! Helpers of the texts 0.2 tests: small requests and the typed problem read back.

#![allow(dead_code)]

use fs_interfaces::rpc::Problem;
use fs_interfaces::texts_capnp::{expect, texts};
use uuid::Uuid;

use super::{read_text, set_id, Got};

pub fn p(error: &capnp::Error) -> Problem {
    Problem::from_error(error).expect("a typed problem")
}

/// The type of a problem below `problems/texts/`.
pub fn short(error: &capnp::Error) -> String {
    let t = p(error).problem_type;
    t.rsplit('/').next().unwrap().to_string()
}

pub fn expect_revision(mut out: expect::Builder<'_>, revision: Option<u64>) {
    match revision {
        Some(r) => out.set_revision(r),
        None => out.set_any(()),
    }
}

/// `edit` the document of `t` to say `markdown`, with `editWith`.
pub async fn edit_with(
    root: &texts::Client,
    id: Uuid,
    revision: Option<u64>,
    document: &str,
    summary: &str,
) -> Result<Got, capnp::Error> {
    let mut req = root.edit_with_request();
    set_id(req.get().init_id(), id);
    expect_revision(req.get().init_expect(), revision);
    req.get().set_document(document);
    req.get().set_summary(summary);
    let answer = req.send().promise.await?;
    Ok(read_text(answer.get()?.get_text()?))
}

/// The revision numbers of one page of history, and the cursor behind it.
pub async fn history(root: &texts::Client, id: Uuid, cursor: &[u8]) -> (Vec<u64>, Vec<u8>) {
    let mut req = root.history_request();
    set_id(req.get().init_id(), id);
    req.get().init_cursor().set_opaque(cursor);
    let answer = req.send().promise.await.unwrap();
    let page = answer.get().unwrap().get_page().unwrap();
    let numbers = page
        .get_items()
        .unwrap()
        .iter()
        .map(fs_interfaces::texts_capnp::revision_info::Reader::get_number)
        .collect();
    (
        numbers,
        page.get_next().unwrap().get_opaque().unwrap().to_vec(),
    )
}

/// The document of a text after `markdown` was written over it (a document JSON to send back).
pub fn document_saying(root: &texts::Client, got: &Got, markdown: &str) -> String {
    let mut doc: fs_text::Document = serde_json::from_str(&got.document).unwrap();
    fs_wiki_core::TextFormat::Markdown.import_external(&mut doc, markdown);
    let _ = root;
    serde_json::to_string(&doc).unwrap()
}

/// Send a request that takes only an id and answer its result.
pub async fn trash(root: &texts::Client, id: Uuid) -> Result<(), capnp::Error> {
    let mut req = root.trash_request();
    set_id(req.get().init_id(), id);
    req.send().promise.await.map(|_| ())
}

/// The ids on one page of the trash, and its cursor.
pub async fn trash_ids(root: &texts::Client, cursor: &[u8]) -> (Vec<Uuid>, Vec<u8>) {
    let mut req = root.list_trash_request();
    req.get().init_cursor().set_opaque(cursor);
    let answer = req.send().promise.await.unwrap();
    let page = answer.get().unwrap().get_page().unwrap();
    let ids = page
        .get_items()
        .unwrap()
        .iter()
        .map(|i| fs_interfaces::rpc::read_uuid(i.get_id().unwrap()).unwrap())
        .collect();
    (ids, page.get_next().unwrap().get_opaque().unwrap().to_vec())
}

/// A folder or space; the node as (id, kind name, name, path).
pub async fn folder(
    root: &texts::Client,
    parent: Option<Uuid>,
    name: &str,
    path: &str,
) -> Result<(Uuid, String, String), capnp::Error> {
    let mut req = root.create_folder_request();
    fs_interfaces::rpc::write_uuid(req.get().init_parent(), parent);
    req.get().set_name(name);
    req.get().set_path(path);
    let answer = req.send().promise.await?;
    let node = answer.get()?.get_node()?;
    Ok((
        fs_interfaces::rpc::read_uuid(node.get_id()?).unwrap(),
        node.get_path()?.to_string().unwrap(),
        format!("{:?}", node.get_kind().unwrap()),
    ))
}

/// The names of the children of `parent`, all pages.
pub async fn names(root: &texts::Client, parent: Option<Uuid>) -> Vec<String> {
    let (mut out, mut cursor) = (vec![], vec![]);
    loop {
        let mut req = root.children_request();
        fs_interfaces::rpc::write_uuid(req.get().init_parent(), parent);
        req.get().init_cursor().set_opaque(&cursor);
        let answer = req.send().promise.await.unwrap();
        let page = answer.get().unwrap().get_page().unwrap();
        out.extend(
            page.get_items()
                .unwrap()
                .iter()
                .map(|n| n.get_name().unwrap().to_string().unwrap()),
        );
        cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
        if cursor.is_empty() {
            return out;
        }
    }
}
