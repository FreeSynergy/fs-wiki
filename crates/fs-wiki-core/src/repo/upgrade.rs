//! Repair of the index of texts stored before 0.3.0.
//!
//! Pattern: Template method (fixed flow: read in chunks, find what lacks the keys, add them, write
//! back, report) — a text written by 0.2.0 has no `path_key` and no `id_key`, so no page can start
//! after it. Only the two index fields are added; the record, the envelope and the other index fields
//! stay as they are, and the document is not read. The flow is idempotent, and one text that cannot be
//! repaired is named in the report and does not stop the rest.

use fs_objects::{sort_key, ObjectId, Order, Query, StoredObject};

use super::handle::{WikiRepo, CONTEXT, TEXT_TYPE};
use super::keys::{ID_KEY, PATH, PATH_KEY};
use crate::error::{Error, Result};

/// Texts read per query.
const CHUNK: usize = 100;

/// What [`WikiRepo::upgrade_index`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexReport {
    /// Texts whose index was completed.
    pub rewritten: usize,
    /// Texts that could not be repaired, each as `<id>: <reason>`. They stay without a place in the
    /// order; a page says so (`Error::Stored`) instead of leaving them out.
    pub failed: Vec<String>,
}

impl WikiRepo {
    /// Complete the index of every text that lacks `path_key` or `id_key`. Run it when the program
    /// starts; it does nothing for texts that are in order. A change is not told to the feed: the
    /// texts did not change.
    pub async fn upgrade_index(&self) -> Result<IndexReport> {
        let _one_at_a_time = self.write_lock.lock().await;
        let mut report = IndexReport::default();
        let mut offset = 0;
        loop {
            let q = Query::new()
                .of_type(TEXT_TYPE)
                .in_context(CONTEXT)
                .ordered(Order::PublishedAsc)
                .limit(CHUNK)
                .offset(offset);
            let chunk = self.reads.query(&q).await?;
            if chunk.is_empty() {
                return Ok(report);
            }
            offset += chunk.len();
            for object in chunk {
                if object.index.contains_key(PATH_KEY) && object.index.contains_key(ID_KEY) {
                    continue;
                }
                let id = object.envelope.id;
                match self.complete(object).await {
                    Ok(()) => report.rewritten += 1,
                    Err(e) => report.failed.push(format!("{id}: {e}")),
                }
            }
        }
    }

    async fn complete(&self, mut object: StoredObject) -> Result<()> {
        let id: ObjectId = object.envelope.id;
        let path = object
            .index
            .get(PATH)
            .and_then(|values| values.first())
            .cloned()
            .ok_or_else(|| Error::Stored("no path in the index".into()))?;
        object.index.insert(PATH_KEY.into(), vec![sort_key(&path)]);
        object.index.insert(ID_KEY.into(), vec![id.to_string()]);
        self.objects.put(object, &self.actor).await?;
        Ok(())
    }
}
