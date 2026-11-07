//! Calls that write.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core. A document travels
//! as its JSON form, which the server reads back before it stores anything.

use fs_interfaces::rpc::write_object_id;
use fs_interfaces::texts_capnp::texts::Client;
use fs_store::ObjectId;
use fs_text::Document;
use fs_wiki_core::{Result, TextEntry};

use crate::problems::{error_of, Subject};
use crate::read::{entry, unreadable};

/// The JSON form of a document, as the contract carries it.
pub(crate) fn document_json(document: &Document) -> Result<String> {
    Ok(serde_json::to_string(document)?)
}

/// `create`.
pub(crate) async fn create(c: &Client, path: String, markdown: String) -> Result<TextEntry> {
    let mut req = c.create_request();
    req.get().set_path(path.as_str());
    req.get().set_markdown(markdown.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    entry(
        answer
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

/// `edit`.
pub(crate) async fn edit(c: &Client, id: ObjectId, document: String) -> Result<TextEntry> {
    let mut req = c.edit_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_document(document.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    entry(
        answer
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

/// `editIf`.
pub(crate) async fn edit_if(
    c: &Client,
    id: ObjectId,
    revision: u64,
    document: String,
) -> Result<TextEntry> {
    let mut req = c.edit_if_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_revision(revision);
    req.get().set_document(document.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Revision(id, revision)))?;
    entry(
        answer
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

/// `move`.
pub(crate) async fn move_to(c: &Client, id: ObjectId, path: String) -> Result<TextEntry> {
    let mut req = c.move_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_path(path.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    entry(
        answer
            .get()
            .and_then(|r| r.get_text())
            .map_err(unreadable)?,
    )
}

/// `moveFolder`.
pub(crate) async fn move_folder(
    c: &Client,
    prefix: String,
    path: String,
) -> Result<Vec<TextEntry>> {
    let mut req = c.move_folder_request();
    req.get().set_prefix(prefix.as_str());
    req.get().set_path(path.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Folder(prefix)))?;
    let moved = answer
        .get()
        .and_then(|r| r.get_moved())
        .map_err(unreadable)?;
    moved.iter().map(entry).collect()
}

/// `delete`.
pub(crate) async fn delete(c: &Client, id: ObjectId) -> Result<()> {
    let mut req = c.delete_request();
    write_object_id(req.get().init_id(), id);
    req.send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Text(id)))?;
    Ok(())
}

/// `importText`.
pub(crate) async fn import_text(
    c: &Client,
    path: String,
    media_type: String,
    text: String,
) -> Result<TextEntry> {
    let mut req = c.import_text_request();
    req.get().set_path(path.as_str());
    req.get().set_media_type(media_type.as_str());
    req.get().set_text(text.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    entry(
        answer
            .get()
            .and_then(|r| r.get_entry())
            .map_err(unreadable)?,
    )
}
