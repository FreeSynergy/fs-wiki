//! The store-backed wiki.
//!
//! Pattern: Repository + Facade — one small type hides the store, the envelope rules and the
//! formats. It sees `ObjectRepository` and `ObjectQuery` only (interface segregation, ADR 0043).
//! Writes take `write_lock`, so the check "is the path free" and the write after it cannot
//! interleave inside one process (the store has no unique index).

use std::sync::Arc;

use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::Store;
use tokio::sync::Mutex;

use super::backend::StoreAdapter;

/// AS2 context of texts; also the first segment of their IRI.
pub const CONTEXT: &str = "texts";
/// AS2 type of texts.
pub const TEXT_TYPE: &str = "Article";
/// Media type of the stored record `{ path, document }`; provisional, ADR 0034 asks for a neutral
/// name for the block specification.
pub const CONTENT_TYPE: &str = "application/vnd.freesynergy.text+json";

/// Texts over `fs-objects`.
pub struct WikiRepo {
    pub(super) objects: Arc<dyn ObjectRepository>,
    pub(super) reads: Arc<dyn ObjectQuery>,
    pub(super) actor: String,
    pub(super) instance: String,
    pub(super) write_lock: Mutex<()>,
}

impl WikiRepo {
    /// A wiki over the whole store, acting as `actor` (an IRI, recorded in the activity log).
    pub fn new(store: Arc<dyn Store>, actor: impl Into<String>) -> Self {
        let adapter = Arc::new(StoreAdapter(store));
        Self::with_backend(adapter.clone(), adapter, actor)
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
            actor: actor.into(),
            instance: "local.invalid".into(),
            write_lock: Mutex::new(()),
        }
    }

    /// The instance host used in the IRI `https://<instance>/texts/<id>` (default `local.invalid`).
    #[must_use]
    pub fn with_instance(mut self, host: impl Into<String>) -> Self {
        self.instance = host.into();
        self
    }
}
