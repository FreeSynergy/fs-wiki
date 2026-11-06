//! Identifiers (ADR 0027: UUIDv7).

pub use uuid::Uuid;

/// Fresh UUIDv7 (time ordered).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}
