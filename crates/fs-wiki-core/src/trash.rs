//! A text in the trash (`text.trash`).
//!
//! Pattern: Value object — plain data. A [`TrashPlace`] is where an item stands in the order of the
//! trash (most recently deleted first, then id), so a page continues after it.

use fs_objects::{ObjectId, Timestamp};

/// One trashed text (`TrashItem`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashItem {
    /// The text.
    pub id: ObjectId,
    /// The path it had when it was deleted.
    pub path: String,
    /// Derived name.
    pub name: String,
    /// When it was moved to the trash.
    pub deleted_at: Timestamp,
    /// Actor that did it.
    pub deleted_by: String,
    /// Bytes of the document JSON.
    pub size: u64,
}

/// Where an item stands in the trash order: newest first, then by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrashPlace {
    /// Milliseconds since the epoch of the deletion.
    pub deleted_ms: i64,
    /// The id.
    pub id: ObjectId,
}

impl TrashItem {
    /// Where this item stands.
    pub fn place(&self) -> TrashPlace {
        TrashPlace {
            deleted_ms: self.deleted_at.timestamp_millis(),
            id: self.id,
        }
    }
}

/// A page of the trash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashPage {
    /// At most the size asked for.
    pub items: Vec<TrashItem>,
    /// Whether items follow.
    pub more: bool,
}
