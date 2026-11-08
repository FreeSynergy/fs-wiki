//! Writing a text and its revision together.
//!
//! Pattern: Template method — one flow for every write of a text: take the next revision number,
//! build the object, store it, store the revision. Revisions are records of their own (type
//! `WikiRevision`, id derived from text and number), written as the bookkeeping actor so the undo of a
//! person's writes does not meet them. `number` is the first one with no record, so a store undo
//! that brings back an old text record, or a compaction of the log, cannot make a number come round
//! again (the log is not where the revisions live).

use fs_objects::{Envelope, ObjectId, StoredObject};
use fs_text::Document;
use serde::{Deserialize, Serialize};

use super::handle::WikiRepo;
use super::ids::revision_id;
use super::record::Flags;
use crate::error::Result;
use crate::texts::TextEntry;

pub(super) const REVISION_TYPE: &str = "WikiRevision";
pub(super) const REVISION_CONTEXT: &str = "wiki-revisions";

/// A revision as stored. `document` is none for a revision removed by `purge_revision`: the number
/// stays used.
#[derive(Serialize, Deserialize)]
pub(super) struct RevisionRecord {
    pub text: ObjectId,
    pub number: u64,
    pub revision: u64,
    pub author: String,
    pub at_ms: i64,
    pub size: u64,
    pub summary: String,
    pub path: String,
    pub restored_from: u64,
    pub document: Option<serde_json::Value>,
    /// A removal that was started and not finished (the log is not yet purged): a retry completes it.
    #[serde(default)]
    pub pending_purge: bool,
}

/// What a write says about itself.
pub(super) struct How<'a> {
    pub summary: &'a str,
    pub restored_from: u64,
    /// New flags; none = the text keeps its own.
    pub flags: Option<Flags>,
}

impl<'a> How<'a> {
    pub(super) fn said(summary: &'a str) -> Self {
        Self {
            summary,
            restored_from: 0,
            flags: None,
        }
    }
}

impl WikiRepo {
    /// Store `doc` as the next revision of text `id` (`prior` = how it is now, none for a new text).
    pub(super) async fn write_text(
        &self,
        id: ObjectId,
        prior: Option<&TextEntry>,
        path: &str,
        doc: &Document,
        how: How<'_>,
    ) -> Result<()> {
        let mut number = self.next_number(id, prior.is_some()).await?;
        if let Some(p) = prior {
            // A state no record has (written before history existed, or whose record failed): keep it.
            if self.record_late(p, number).await? {
                number += 1;
            }
        }
        let flags = how.flags.unwrap_or_else(|| Flags::of(prior));
        let obj = self.build_object(id, prior, path, doc, (flags, number))?;
        let revision = obj.content["revision"].as_u64().unwrap_or(0);
        self.objects.put(obj, &self.actor).await?;
        let record = RevisionRecord {
            text: id,
            number,
            revision,
            author: self.actor.clone(),
            at_ms: chrono::Utc::now().timestamp_millis(),
            size: serde_json::to_string(doc)?.len() as u64,
            summary: how.summary.to_string(),
            path: path.to_string(),
            restored_from: how.restored_from,
            document: Some(serde_json::to_value(doc)?),
            pending_purge: false,
        };
        self.put_revision(&record).await
    }

    /// Record `text` as revision `next` if the newest record (`next - 1`) is not its state: none at all
    /// (a text from before history), or another revision (a failed write, a store undo). True if written.
    pub(super) async fn record_late(&self, text: &TextEntry, next: u64) -> Result<bool> {
        let newest = match next.checked_sub(1).filter(|n| *n >= 1) {
            Some(n) => self.revision_record(text.id, n).await?,
            None => None,
        };
        let summary = match &newest {
            None => "state before history began",
            Some(r) if r.revision != text.revision => {
                "state recorded late (a failed write, or brought back by a store undo)"
            }
            Some(_) => return Ok(false),
        };
        let record = RevisionRecord {
            text: text.id,
            number: next,
            revision: text.revision,
            author: String::new(),
            at_ms: chrono::Utc::now().timestamp_millis(),
            size: serde_json::to_string(&text.document)?.len() as u64,
            summary: summary.into(),
            path: text.path.clone(),
            restored_from: 0,
            document: Some(serde_json::to_value(&text.document)?),
            pending_purge: false,
        };
        self.put_revision(&record).await?;
        Ok(true)
    }

    /// The first revision number of `id` that has no record yet, starting from the hint the text
    /// record keeps.
    pub(super) async fn next_number(&self, id: ObjectId, existing: bool) -> Result<u64> {
        let hint = if existing {
            self.objects
                .get(id)
                .await?
                .and_then(|o| o.content["hint"].as_u64())
                .unwrap_or(0)
        } else {
            0
        };
        let mut number = hint.max(1);
        while self.objects.get(revision_id(id, number)).await?.is_some() {
            number += 1;
        }
        Ok(number)
    }

    pub(super) async fn put_revision(&self, record: &RevisionRecord) -> Result<()> {
        let id = revision_id(record.text, record.number);
        let mut env = Envelope::new(REVISION_TYPE, format!("{}#{}", record.text, record.number));
        env.id = id;
        env.context = Some(REVISION_CONTEXT.into());
        let object = StoredObject::new(env, "application/json", serde_json::to_value(record)?);
        self.objects.put(object, &self.aux_actor()).await?;
        Ok(())
    }

    pub(super) async fn revision_record(
        &self,
        text: ObjectId,
        number: u64,
    ) -> Result<Option<RevisionRecord>> {
        let Some(object) = self.objects.get(revision_id(text, number)).await? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_value(object.content)?))
    }
}
