//! Text <-> stored object: the record, the envelope and the index fields.
//!
//! Pattern: Adapter — maps a path and a [`Document`] onto the AS2 `Article` envelope and index
//! fields that `fs-store` understands (ADR 0020). The content is the record `{ path, document }`
//! as JSON, so the stored object is complete without its index. Index fields (all derived, never
//! the truth): `path`; `link` = the path every internal link means (a relative link is resolved against its page), which answers backlinks
//! in one query; `block` = the plain text of each block, which answers `find` in the store.

use fs_store::{Envelope, IndexFields, ObjectId, StoredObject};
use fs_text::Document;
use serde::{Deserialize, Serialize};

use super::definitions::definition_links;
use super::handle::{WikiRepo, CONTENT_TYPE, CONTEXT, TEXT_TYPE};
use super::name::derive_name;
use super::target::written;
use crate::error::{Error, Result};
use crate::hidden::is_hidden_block;
use crate::texts::TextEntry;

/// What is stored as the content of a text.
#[derive(Serialize, Deserialize)]
struct Record {
    path: String,
    document: Document,
    /// Changes with every write and never repeats; records written before it existed read as 0.
    #[serde(default)]
    revision: u64,
}

impl WikiRepo {
    /// The object to store for `doc` at `path`; `prior` keeps id, publication time and IRI and
    /// gives the next revision.
    pub(super) fn build_object(
        &self,
        id: ObjectId,
        prior: Option<&TextEntry>,
        path: &str,
        doc: &Document,
    ) -> Result<StoredObject> {
        let mut env = prior.map_or_else(|| self.fresh_envelope(id), |p| p.envelope.clone());
        env.name = derive_name(doc, path);
        let record = Record {
            path: path.to_string(),
            document: doc.clone(),
            revision: fresh_revision(),
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
        revision: record.revision,
        document: record.document,
    })
}

fn index_fields(path: &str, doc: &Document) -> IndexFields {
    let mut index = IndexFields::new();
    index.insert("path".into(), vec![path.to_string()]);
    let mut links: Vec<String> = vec![];
    for block in &doc.blocks {
        let marked = block.marks.iter().filter_map(|m| m.link().cloned());
        for link in marked.chain(definition_links(block)) {
            let Some(w) = written(&link, path) else {
                continue;
            };
            for candidate in w.candidates() {
                if !links.contains(&candidate) {
                    links.push(candidate);
                }
            }
        }
    }
    if !links.is_empty() {
        index.insert("link".into(), links);
    }
    let blocks: Vec<String> = doc
        .blocks
        .iter()
        .filter(|b| !is_hidden_block(b))
        .map(|b| b.text.to_string().trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    if !blocks.is_empty() {
        index.insert("block".into(), blocks);
    }
    index
}

/// A revision no earlier write of any text had and no later one will have: random bits of a
/// time-ordered id, never 0 (that is for records from before revisions). Not a counter, so a
/// store undo that brings back an old record cannot make a revision come round again.
fn fresh_revision() -> u64 {
    ObjectId::new().as_uuid().as_u64_pair().1 | 1
}
