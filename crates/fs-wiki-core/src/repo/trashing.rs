//! The trash: out of sight, kept with its history, restored or removed for good.
//!
//! Pattern: Command — a trashed text is the same object with another context (`texts-trash`) and a
//! mark in its record; every read of live texts asks for the context `texts`, so none of them knows
//! about the trash. Its path, link and block indexes are dropped (it is not found, not linked to, its
//! path is free); `purge` removes its revisions, the object and their log entries.

use fs_objects::{IndexFields, ObjectId, Query, StoredObject};

use super::handle::{WikiRepo, CONTEXT, TEXT_TYPE, TRASH_CONTEXT};
use super::ids::revision_id;
use super::keys::{ID_KEY, TRASH_PATH};
use super::record::{record_of, Flags};
use super::revise::{How, REVISION_CONTEXT, REVISION_TYPE};
use crate::error::{Error, Result};
use crate::feed::TextChange;
use crate::path::TextPath;
use crate::texts::TextEntry;
use crate::trash::{TrashItem, TrashPage, TrashPlace};

fn item_of(o: &StoredObject) -> Option<TrashItem> {
    let at = o.content["trashed"]["at_ms"].as_i64()?;
    Some(TrashItem {
        id: o.envelope.id,
        path: o.content["path"].as_str()?.to_string(),
        name: o.envelope.name.clone(),
        deleted_at: chrono::DateTime::from_timestamp_millis(at).unwrap_or_default(),
        deleted_by: o.content["trashed"]["by"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        size: o.content["document"].to_string().len() as u64,
    })
}

/// The order of the trash: most recently deleted first, then by id.
fn before(a: &TrashPlace, b: &TrashPlace) -> std::cmp::Ordering {
    b.deleted_ms.cmp(&a.deleted_ms).then(a.id.cmp(&b.id))
}

impl WikiRepo {
    /// The object of a trashed text.
    async fn trashed_object(&self, id: ObjectId) -> Result<StoredObject> {
        match self.objects.get(id).await? {
            Some(o) if o.envelope.object_type == TEXT_TYPE => {
                if o.envelope.context.as_deref() == Some(TRASH_CONTEXT) {
                    Ok(o)
                } else {
                    Err(Error::NotInTrash(id))
                }
            }
            _ => Err(Error::NotFound(id)),
        }
    }

    pub(super) async fn trash_locked(&self, id: ObjectId) -> Result<TrashItem> {
        self.text(id).await?.ok_or(Error::NotFound(id))?;
        let mut object = self.objects.get(id).await?.ok_or(Error::NotFound(id))?;
        let path = object.content["path"].as_str().unwrap_or("").to_string();
        let at = chrono::Utc::now().timestamp_millis();
        object.content["trashed"] = serde_json::json!({ "at_ms": at, "by": self.actor });
        object.envelope.context = Some(TRASH_CONTEXT.into());
        let mut index = IndexFields::new();
        index.insert(ID_KEY.into(), vec![id.to_string()]);
        index.insert(TRASH_PATH.into(), vec![path]);
        object.index = index;
        self.objects.put(object.clone(), &self.actor).await?;
        self.feed.notify(&TextChange::Deleted(id));
        item_of(&object).ok_or_else(|| Error::Stored(format!("trashed text {id} has no mark")))
    }

    pub(super) async fn trash_page(
        &self,
        after: Option<&TrashPlace>,
        size: usize,
    ) -> Result<TrashPage> {
        let q = Query::new().of_type(TEXT_TYPE).in_context(TRASH_CONTEXT);
        let mut items: Vec<TrashItem> = self
            .reads
            .query(&q)
            .await?
            .iter()
            .filter_map(item_of)
            .collect();
        items.sort_by(|a, b| before(&a.place(), &b.place()));
        if let Some(after) = after {
            items.retain(|i| before(after, &i.place()).is_lt());
        }
        let more = items.len() > size;
        items.truncate(size);
        Ok(TrashPage { items, more })
    }

    pub(super) async fn restore_trashed_locked(
        &self,
        id: ObjectId,
        path: &str,
    ) -> Result<TextEntry> {
        let object = self.trashed_object(id).await?;
        let (mut entry, _) = record_of(object)?;
        let target = if path.is_empty() {
            entry.path.clone()
        } else {
            path.to_string()
        };
        let target = TextPath::parse(&target)?;
        self.ensure_free(&target, None).await?;
        entry.envelope.context = Some(CONTEXT.into());
        let doc = entry.document.clone();
        let how = How {
            summary: "restored from trash",
            restored_from: 0,
            flags: Some(Flags::of(Some(&entry))),
        };
        self.write_text(id, Some(&entry), target.as_str(), &doc, how)
            .await?;
        self.feed.notify(&TextChange::Created(id));
        self.entry(id).await
    }

    pub(super) async fn purge_locked(&self, id: ObjectId) -> Result<()> {
        let mut object = self.trashed_object(id).await?;
        let purger = self.purger("text.trash")?;
        // Every record of the text, found by name (`<text>#<number>`), not by counting up from a hint:
        // a store undo moves the hint back and a late record does not raise it, so records can lie above
        // a gap. The highest number seen is written into the trashed text before anything is deleted, so
        // a retry purges the log of records that are already gone.
        let q = Query::new()
            .of_type(REVISION_TYPE)
            .in_context(REVISION_CONTEXT)
            .name_prefix(format!("{id}#"));
        let found = self.reads.query(&q).await?;
        let seen = found
            .iter()
            .filter_map(|o| o.content["number"].as_u64())
            .max();
        let before = object.content["high"].as_u64().unwrap_or(0);
        let probe = self.next_number(id, true).await?.saturating_sub(1);
        let high = [
            before,
            object.content["hint"].as_u64().unwrap_or(0),
            probe,
            seen.unwrap_or(0),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        if high > before {
            object.content["high"] = high.into();
            self.objects.put(object, &self.actor).await?;
        }
        for o in &found {
            self.objects
                .delete(o.envelope.id, &self.aux_actor())
                .await?;
        }
        let mut cleaned = true;
        for number in 1..=high {
            cleaned &= purger.purge_log(revision_id(id, number)).await?;
        }
        // The log of the text first: the delete after it is logged without images (the object is marked
        // purged), and a stop between the two leaves a text that purge can still be repeated on.
        cleaned &= purger.purge_log(id).await?;
        self.objects.delete(id, &self.actor).await?;
        if !cleaned {
            purger.clean_files().await?;
        }
        Ok(())
    }

    /// Whether a trashed text lies at or below `prefix/` (it blocks the deletion of that folder).
    pub(super) async fn trash_below(&self, prefix: &str) -> Result<bool> {
        let low = format!("{prefix}/");
        let q = Query::new()
            .of_type(TEXT_TYPE)
            .in_context(TRASH_CONTEXT)
            .index_between(TRASH_PATH, low.clone(), format!("{low}\u{10FFFF}"))
            .limit(1);
        Ok(!self.reads.query(&q).await?.is_empty())
    }
}
