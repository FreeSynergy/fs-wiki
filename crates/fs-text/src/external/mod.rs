//! Changes from outside (ADR 0042 §5): an exported file edited elsewhere is an **import**.
//!
//! Pattern: Command producer (diff -> ops) — the new text is compared with the stored state
//! and the differences become ordinary [`crate::Op`]s, so marks move with the text.

mod blocks;
mod diff;
mod plan;

pub use diff::{diff_edits, TextEdit};
pub use plan::{import_external, plan_external_import, plan_external_import_with};
