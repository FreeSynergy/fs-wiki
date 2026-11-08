//! The store-backed wiki.
//!
//! Pattern: Repository + Facade — one small type hides the store, the envelope rules and the
//! formats. It sees `ObjectRepository` and `ObjectQuery` only (interface segregation, ADR 0043).
//! Writes take `write_lock`, so the check "is the path free" and the write after it cannot
//! interleave inside one process (the store has no unique index).

use std::sync::Arc;

use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{Store, StoreInfo};
use tokio::sync::Mutex;

use super::backend::{LogPurge, StoreAdapter};
use crate::feed::Registry;

/// AS2 context of texts; also the first segment of their IRI.
pub const CONTEXT: &str = "texts";
/// Context of trashed texts: every read of live texts asks for `CONTEXT`, so a trashed text is out of
/// sight without any of them knowing about the trash.
pub(super) const TRASH_CONTEXT: &str = "texts-trash";
/// AS2 type of texts.
pub const TEXT_TYPE: &str = "Article";
/// Media type of the stored record `{ path, document }`; provisional, ADR 0034 asks for a neutral
/// name for the block specification.
pub const CONTENT_TYPE: &str = "application/vnd.freesynergy.text+json";

/// Texts over `fs-objects`.
pub struct WikiRepo {
    pub(super) objects: Arc<dyn ObjectRepository>,
    pub(super) reads: Arc<dyn ObjectQuery>,
    /// What the store says about itself; none for a backend that is not a whole store.
    pub(super) info: Option<Arc<dyn StoreInfo>>,
    /// Removes an object from the log; none for a backend that is not a whole store.
    pub(super) purger: Option<Arc<dyn LogPurge>>,
    pub(super) actor: String,
    pub(super) instance: String,
    pub(super) write_lock: Mutex<()>,
    /// The subject of the change feed (`text.subscribe`); told after every write.
    pub(super) feed: Arc<Registry>,
}

impl WikiRepo {
    /// A wiki over the whole store, acting as `actor` (an IRI, recorded in the activity log).
    pub fn new(store: Arc<dyn Store>, actor: impl Into<String>) -> Self {
        let adapter = Arc::new(StoreAdapter(store));
        let mut repo = Self::with_backend(adapter.clone(), adapter.clone(), actor);
        repo.purger = Some(adapter.clone());
        repo.with_info(adapter)
    }

    /// A wiki over the narrow store traits only (a remote or fake backend can supply them).
    pub fn with_backend(
        objects: Arc<dyn ObjectRepository>,
        reads: Arc<dyn ObjectQuery>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            objects,
            reads,
            info: None,
            purger: None,
            actor: actor.into(),
            instance: "local.invalid".into(),
            write_lock: Mutex::new(()),
            feed: Arc::default(),
        }
    }

    /// Say whose store this is: [`Texts::identity`](crate::Texts::identity) answers from it.
    #[must_use]
    pub fn with_info(mut self, info: Arc<dyn StoreInfo>) -> Self {
        self.info = Some(info);
        self
    }

    /// The instance host used in the IRI `https://<instance>/texts/<id>` (default `local.invalid`).
    #[must_use]
    pub fn with_instance(mut self, host: impl Into<String>) -> Self {
        self.instance = host.into();
        self
    }
}
