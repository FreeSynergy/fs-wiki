//! Undo / redo log (ADR 0042 §10).
//!
//! Pattern: Command history (Memento of ops) — steps group commands with their inverses.

mod compact;
mod log;
mod retention;
mod step;

pub use log::History;
pub use retention::Retention;
pub use step::Step;
