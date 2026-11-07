//! What a file looked like when it was last imported or exported.
//!
//! Pattern: Memento + Repository — one small record per file, in the same store as the texts
//! (so one store per folder keeps the records of that folder). With it a restart can tell a file
//! that did not change (nothing to do, whatever the database holds), a file changed outside, a
//! text that was moved or deleted since (the record still names the file), and a new file. The
//! hashes are opaque to the core; the caller computes them.

use async_trait::async_trait;
use fs_store::ObjectId;
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

/// The file records of a wiki.
#[async_trait]
pub trait Origins: Send + Sync {
    /// Every record, in the order of the file paths.
    async fn origins(&self) -> Result<Vec<FileOrigin>>;
    /// Store `origin`, replacing the record of the same file.
    async fn set_origin(&self, origin: &FileOrigin) -> Result<()>;
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
