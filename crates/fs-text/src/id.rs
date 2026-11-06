//! Identifiers (ADR 0027: UUIDv7).
//!
//! Pattern: Factory + Newtype re-export — `new_id` is the single place that creates ids, and
//! callers only see [`Uuid`], so the id scheme can change without touching the model.

pub use uuid::Uuid;

/// Fresh UUIDv7 (time ordered).
#[must_use]
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}
