//! What a file looked like when it was last imported or exported.
//!
//! Pattern: Memento + Repository — one small record per file, in the same store as the texts
//! (so one store per folder keeps the records of that folder). With it a restart can tell a file
//! that did not change (nothing to do, whatever the database holds), a file changed outside, a
//! text that was moved or deleted since (the record still names the file), and a new file. The
//! hashes are opaque to the core; the caller computes them.

use async_trait::async_trait;
use fs_objects::{ObjectId, StoreId};
use serde::{Deserialize, Serialize};

use super::handle::WikiRepo;
use crate::error::Result;

const KIND: &str = "FileOrigin";

/// The record of one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOrigin {
    /// Path of the file relative to the folder, with `/`; the key.
    pub file: String,
    /// Hash of the file as it was last imported or exported.
    pub hash: String,
    /// The text that file was imported into (it keeps its id when moved).
    pub text: ObjectId,
    /// Hash of the Markdown of that text right after the import or export.
    pub text_hash: String,
    /// The text was deleted on purpose and the person knows: the file is no longer reported as
    /// missing from the wiki. Records from before this field read as `false`.
    #[serde(default)]
    pub deleted: bool,
}

/// What `Origins::belong_to` found that is not the selected store's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Foreign {
    /// Records from before the store was recorded: nobody knows whose they are.
    pub legacy: Vec<FileOrigin>,
    /// How many records are kept for other stores.
    pub others: usize,
}

/// The file records of a wiki.
#[async_trait]
pub trait Origins: Send + Sync {
    /// Every record, in the order of the file paths.
    async fn origins(&self) -> Result<Vec<FileOrigin>>;
    /// Store `origin`, replacing the record of the same file.
    async fn set_origin(&self, origin: &FileOrigin) -> Result<()>;
    /// Say which store the caller is connected to (`None`: it has no identity); `origins` and
    /// `set_origin` answer for that store from now on. Records are kept for each store they were
    /// made for, so going back to a store brings its records (and its `deleted` marks) back. Records
    /// inside the store need no answer: they cannot be of another one.
    async fn belong_to(&self, _store: Option<StoreId>) -> Result<Foreign> {
        Ok(Foreign::default())
    }
    /// Take over the records from before any store was recorded (`Foreign::legacy`) as those of the
    /// store selected by `belong_to`. Only for the caller that has checked they fit.
    async fn adopt_legacy(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl Origins for WikiRepo {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        self.aux_all(KIND).await
    }

    async fn set_origin(&self, origin: &FileOrigin) -> Result<()> {
        let _write = self.write_lock.lock().await;
        self.aux_put(KIND, &origin.file, origin).await
    }
}
