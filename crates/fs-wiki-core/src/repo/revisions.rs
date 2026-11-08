//! Reading, restoring and removing revisions of a text.
//!
//! Pattern: Repository (read model) + Command — a revision is found by `get` on its derived id, newest
//! first by counting down from the latest number, so a page costs what it shows and not the size
//! of the wiki. `restore` is an ordinary write; `purge_revision` takes the revision out of the store
//! and out of the log (the log keeps before/after images of every object, so the text's own
//! log goes too: see DESIGN.md "texts 0.2").

use fs_objects::ObjectId;

use super::handle::{WikiRepo, TEXT_TYPE};
use super::ids::revision_id;
use super::record::record_of;
use super::revise::{How, RevisionRecord};
use crate::error::{Error, Result};
use crate::history::{Expect, RevisionContent, RevisionInfo, RevisionPage};
use crate::limits::{MAX_HISTORY_PAGE, MAX_SUMMARY_BYTES};
use crate::texts::TextEntry;

fn info_of(r: &RevisionRecord) -> RevisionInfo {
    RevisionInfo {
        text: r.text,
        number: r.number,
        revision: r.revision,
        author: r.author.clone(),
        at: chrono::DateTime::from_timestamp_millis(r.at_ms).unwrap_or_default(),
        size: r.size,
        summary: r.summary.clone(),
        path: r.path.clone(),
        restored_from: r.restored_from,
    }
}

impl WikiRepo {
    /// The text `id`, in the trash or not (history and purge reach both).
    pub(super) async fn text_any(&self, id: ObjectId) -> Result<TextEntry> {
        match self.objects.get(id).await? {
            Some(o) if o.envelope.object_type == TEXT_TYPE => Ok(record_of(o)?.0),
            _ => Err(Error::NotFound(id)),
        }
    }

    /// The number of the newest revision of `id` (0 = none yet).
    async fn latest_number(&self, id: ObjectId) -> Result<u64> {
        Ok(self.next_number(id, true).await?.saturating_sub(1))
    }

    pub(super) async fn history_of(
        &self,
        id: ObjectId,
        before: Option<u64>,
        size: usize,
    ) -> Result<RevisionPage> {
        let text = self.text_any(id).await?;
        let size = size.clamp(1, MAX_HISTORY_PAGE);
        self.catch_up(&text).await?;
        let latest = self.latest_number(id).await?;
        let mut number = before.map_or(latest, |b| b.saturating_sub(1).min(latest));
        let mut items = vec![];
        while number >= 1 && items.len() <= size {
            if let Some(r) = self.revision_record(id, number).await? {
                if r.document.is_some() {
                    items.push(info_of(&r));
                }
            }
            number -= 1;
        }
        let more = items.len() > size;
        items.truncate(size);
        Ok(RevisionPage { items, more })
    }

    /// A store undo (outside this API) can bring an older record of the text back: then the newest
    /// revision is not what the text says. History says so with a revision of its own, written as a
    /// record only (the text is not touched, so no `expect` of a writer is spoiled).
    async fn catch_up(&self, text: &TextEntry) -> Result<()> {
        let stale =
            |r: &Option<RevisionRecord>| r.as_ref().is_some_and(|r| r.revision != text.revision);
        let latest = self.latest_number(text.id).await?;
        if !stale(&self.revision_record(text.id, latest).await?) {
            return Ok(());
        }
        let _write = self.write_lock.lock().await;
        let latest = self.latest_number(text.id).await?;
        if !stale(&self.revision_record(text.id, latest).await?) {
            return Ok(());
        }
        let record = RevisionRecord {
            text: text.id,
            number: latest + 1,
            revision: text.revision,
            author: String::new(),
            at_ms: chrono::Utc::now().timestamp_millis(),
            size: serde_json::to_string(&text.document)?.len() as u64,
            summary: "state brought back outside this interface (store undo)".into(),
            path: text.path.clone(),
            restored_from: 0,
            document: Some(serde_json::to_value(&text.document)?),
        };
        self.put_revision(&record).await
    }

    pub(super) async fn revision_of(
        &self,
        id: ObjectId,
        number: u64,
    ) -> Result<Option<RevisionContent>> {
        self.text_any(id).await?;
        let Some(r) = self.revision_record(id, number).await? else {
            return Ok(None);
        };
        let Some(json) = r.document.clone() else {
            return Ok(None);
        };
        Ok(Some(RevisionContent {
            info: info_of(&r),
            document: serde_json::from_value(json)?,
        }))
    }

    pub(super) async fn restore_locked(
        &self,
        id: ObjectId,
        number: u64,
        expect: Expect,
    ) -> Result<TextEntry> {
        let now = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Self::check_expect(&now, expect)?;
        let old = self
            .revision_of(id, number)
            .await?
            .ok_or(Error::NoSuchRevision { id, number })?;
        let mut doc = old.document;
        doc.id = id.as_uuid();
        let summary = format!("restored revision {number}");
        let how = How {
            summary: &summary,
            restored_from: number,
            flags: None,
        };
        self.write_text(id, Some(&now), &now.path, &doc, how)
            .await?;
        self.feed.notify(&crate::feed::TextChange::Updated(id));
        self.entry(id).await
    }

    /// `Err(Conflict)` unless `expect` is met by `text`.
    pub(super) fn check_expect(text: &TextEntry, expect: Expect) -> Result<()> {
        match expect {
            Expect::Revision(r) if r != text.revision => Err(Error::Conflict {
                id: text.id,
                expected: r,
                found: text.revision,
            }),
            _ => Ok(()),
        }
    }

    pub(super) fn check_summary(summary: &str) -> Result<()> {
        if summary.len() > MAX_SUMMARY_BYTES {
            return Err(Error::Invalid {
                issue: "summary-too-long",
                detail: format!("a summary is at most {MAX_SUMMARY_BYTES} bytes"),
            });
        }
        Ok(())
    }

    pub(super) async fn purge_revision_locked(&self, id: ObjectId, number: u64) -> Result<()> {
        self.text_any(id).await?;
        let gone = || Error::NoSuchRevision { id, number };
        let record = self.revision_record(id, number).await?.ok_or_else(gone)?;
        if record.document.is_none() {
            return Err(gone());
        }
        if number == self.latest_number(id).await? {
            return Err(Error::Invalid {
                issue: "current-revision",
                detail: "the current revision cannot be removed; edit the text first".into(),
            });
        }
        let purger = self.purger()?;
        let rev = revision_id(id, number);
        self.objects.delete(rev, &self.aux_actor()).await?;
        let mut cleaned = purger.purge_log(rev).await?;
        // The text's own log keeps the old document as an image of each change.
        cleaned &= purger.purge_log(id).await?;
        let tombstone = RevisionRecord {
            document: None,
            summary: String::new(),
            ..record
        };
        self.put_revision(&tombstone).await?;
        if !cleaned {
            purger.clean_files().await?;
        }
        Ok(())
    }
}
