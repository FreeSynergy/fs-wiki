//! Text <-> stored object: the record, the envelope and the index fields.
//!
//! Pattern: Adapter — maps a path and a [`Document`] onto the AS2 `Article` envelope and index
//! fields that `fs-store` understands (ADR 0020). The content is the record `{ path, document }`
//! as JSON, so the stored object is complete without its index. Index fields (all derived, never
//! the truth): `path`; `link` = the page part of every internal link, which answers backlinks
//! in one query; `block` = the plain text of each block, which answers `find` in the store.

use fs_store::{Envelope, IndexFields, ObjectId, StoredObject};
use fs_text::{extract_links, Document};
use serde::{Deserialize, Serialize};

use super::handle::{WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
use super::name::derive_name;
use crate::error::{Error, Result};
use crate::path::page_of;
use crate::texts::TextEntry;

/// What is stored as the content of a text.
#[derive(Serialize, Deserialize)]
struct Record {
    path: String,
    document: Document,
}

impl WikiRepo {
    /// The object to store for `doc` at `path`; `existing` keeps id, publication time and IRI.
    pub(super) fn build_object(
        &self,
        id: ObjectId,
        existing: Option<&Envelope>,
        path: &str,
        doc: &Document,
    ) -> Result<StoredObject> {
        let mut env = existing.cloned().unwrap_or_else(|| self.fresh_envelope(id));
        env.name = derive_name(doc, path);
        let record = Record {
            path: path.to_string(),
            document: doc.clone(),
        };
        let mut obj = StoredObject::new(env, CONTENT_TYPE, serde_json::to_value(record)?);
        obj.index = index_fields(path, doc);
        Ok(obj)
    }

    fn fresh_envelope(&self, id: ObjectId) -> Envelope {
        let mut e = Envelope::new(TEXT_TYPE, "text");
        e.id = id;
        e.iri = Some(format!("https://{}/{CONTEXT}/{id}", self.instance));
        e.context = Some(CONTEXT.into());
        e.attributed_to = Some(self.actor.clone());
        e.to = vec![self.actor.clone()];
        e
    }
}

/// Read a stored object back as a text.
pub(super) fn entry_of(o: StoredObject) -> Result<TextEntry> {
    let id = o.envelope.id;
    if o.envelope.object_type != TEXT_TYPE || o.content_type != CONTENT_TYPE {
        return Err(Error::NotAText(id));
    }
    let record: Record = serde_json::from_value(o.content)?;
    Ok(TextEntry {
        id,
        envelope: o.envelope,
        path: record.path,
        document: record.document,
    })
}

fn index_fields(path: &str, doc: &Document) -> IndexFields {
    let mut index = IndexFields::new();
    index.insert("path".into(), vec![path.to_string()]);
    let mut links: Vec<String> = vec![];
    for row in extract_links(doc).into_iter().filter(|r| r.internal) {
        let page = page_of(&row.path);
        if !page.is_empty() && !links.iter().any(|l| l == page) {
            links.push(page.to_string());
        }
    }
    if !links.is_empty() {
        index.insert("link".into(), links);
    }
    let blocks: Vec<String> = doc
        .blocks
        .iter()
        .map(|b| b.text.to_string().trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if !blocks.is_empty() {
        index.insert("block".into(), blocks);
    }
    index
}
