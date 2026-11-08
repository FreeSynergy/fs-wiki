//! A stand-in for the hub way of purging, for tests that are not about the hub: it purges in the store
//! itself, which is what `sync_then_purge` does once the hub has everything.

#![allow(dead_code)]

use std::sync::Arc;

use async_trait::async_trait;
use fs_objects::{ObjectId, Store};
use fs_wiki_core::{LogPurge, PurgeOutcome, Refusal};

pub struct Direct(pub Arc<dyn Store>);

#[async_trait]
impl LogPurge for Direct {
    async fn purge(&self, id: ObjectId) -> Result<PurgeOutcome, Refusal> {
        let r = self
            .0
            .purge_object(id)
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))?;
        Ok(PurgeOutcome {
            file_cleaned: r.file_cleaned,
        })
    }

    async fn clean_files(&self) -> Result<bool, Refusal> {
        self.0
            .clean_files()
            .await
            .map_err(|e| Refusal::Failed(e.to_string()))
    }
}
