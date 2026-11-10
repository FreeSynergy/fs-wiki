//! Small records next to the texts: the move record and the file origins.
//!
//! Pattern: Repository (generic part) — one JSON record per key and kind, stored as an object of
//! its own type in the same store, so it lives and dies with the texts it describes. They are
//! written as the actor `<actor>#bookkeeping`, so the undo of a person's own writes never meets
//! them. Read by index `key`.

use fs_objects::{Envelope, ObjectId, Query, StoredObject};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::handle::WikiRepo;
use crate::error::Result;

/// Context of all bookkeeping objects (texts have `texts`).
const AUX_CONTEXT: &str = "wiki-bookkeeping";
const AUX_CONTENT: &str = "application/json";
const PAGE: usize = 1000;
/// The record that says a file clean-up after a purge is still to be done.
const CLEAN_KIND: &str = "CleanPending";
const CLEAN_KEY: &str = "file";

fn query(kind: &str) -> Query {
    Query::new()
        .of_type(kind)
        .in_context(AUX_CONTEXT)
        .order_by_index("key")
}

impl WikiRepo {
    pub(super) fn aux_actor(&self) -> String {
        format!("{}#bookkeeping", self.actor)
    }

    /// The record of `kind` with `key`, with the id it is stored under.
    pub(super) async fn aux_get<T: DeserializeOwned>(
        &self,
        kind: &str,
        key: &str,
    ) -> Result<Option<(ObjectId, T)>> {
        // One key names one record: no order (it would sort every record of the kind for one row).
        let one = Query::new()
            .of_type(kind)
            .in_context(AUX_CONTEXT)
            .with_index("key", key)
            .limit(1);
        let found = self.reads.query(&one).await?;
        found.into_iter().next().map(decode).transpose()
    }

    /// Every record of `kind`, in key order.
    pub(super) async fn aux_all<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        let mut out = vec![];
        loop {
            let mut q = query(kind).offset(out.len());
            q.limit = Some(PAGE);
            let page = self.reads.query(&q).await?;
            let n = page.len();
            for object in page {
                out.push(decode(object)?.1);
            }
            if n < PAGE {
                return Ok(out);
            }
        }
    }

    /// Store `value` as the record of `kind` with `key` (replaces the one there).
    pub(super) async fn aux_put<T: Serialize + Sync>(
        &self,
        kind: &str,
        key: &str,
        value: &T,
    ) -> Result<()> {
        let id = match self.aux_get::<serde_json::Value>(kind, key).await? {
            Some((id, _)) => id,
            None => ObjectId::new(),
        };
        let mut env = Envelope::new(kind, key);
        env.id = id;
        env.context = Some(AUX_CONTEXT.into());
        let mut object = StoredObject::new(env, AUX_CONTENT, serde_json::to_value(value)?);
        object.index.insert("key".into(), vec![key.to_string()]);
        self.objects.put(object, &self.aux_actor()).await?;
        Ok(())
    }
}

fn decode<T: DeserializeOwned>(object: StoredObject) -> Result<(ObjectId, T)> {
    Ok((object.envelope.id, serde_json::from_value(object.content)?))
}

impl WikiRepo {
    /// What removes an object from the log, or the explicit "not possible" of a backend without it.
    pub(super) fn purger(
        &self,
        capability: &str,
    ) -> Result<&std::sync::Arc<dyn crate::purge::LogPurge>> {
        self.purger.as_ref().ok_or_else(|| {
            fs_interfaces::Unsupported::new(
                capability,
                "this backend cannot remove an object from its log",
            )
            .into()
        })
    }
}

impl WikiRepo {
    /// Purge the log of `id` through the port.
    pub(super) async fn purge_log(&self, capability: &str, id: ObjectId) -> Result<()> {
        self.purger(capability)?.purge(id).await?;
        Ok(())
    }

    /// Before the first log is purged: finish an earlier clean-up and write the reminder that a clean-up is
    /// due, so a stop between the purge and the clean-up cannot lose it.
    pub(super) async fn begin_purge(&self, capability: &str) -> Result<()> {
        self.purger(capability)?;
        let _ = self.retry_file_cleanup().await;
        self.aux_put(CLEAN_KIND, CLEAN_KEY, &true).await
    }

    /// After the last delete or tombstone: clean the file **always**. The purge of each log cleans the file
    /// too, but the rows the wiki deletes or overwrites afterwards stay in free pages and in the
    /// write-ahead log until this. If it fails the reminder stays; the purge itself is a success (its
    /// object is gone, a retry could only say "not found") and the next purge, `retry_file_cleanup` or the
    /// server start repeats the clean-up.
    pub(super) async fn finish_purge(&self, capability: &str) -> Result<()> {
        if matches!(self.purger(capability)?.clean_files().await, Ok(true)) {
            self.aux_put(CLEAN_KIND, CLEAN_KEY, &false).await?;
        }
        Ok(())
    }
}

impl WikiRepo {
    /// Repeat a file clean-up that a purge could not finish. True if nothing is pending now. A server
    /// calls it at start; every purge calls it first.
    pub async fn retry_file_cleanup(&self) -> Result<bool> {
        let pending = self
            .aux_get::<bool>(CLEAN_KIND, CLEAN_KEY)
            .await?
            .is_some_and(|(_, p)| p);
        if !pending {
            return Ok(true);
        }
        let Some(port) = self.purger.as_ref() else {
            return Ok(false);
        };
        let done = matches!(port.clean_files().await, Ok(true));
        if done {
            self.aux_put(CLEAN_KIND, CLEAN_KEY, &false).await?;
        }
        Ok(done)
    }
}
