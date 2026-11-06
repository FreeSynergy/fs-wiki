//! Marks: one mechanism for formatting, links, names, variables and explanations (ADR 0033).
//!
//! Pattern: Registry + Strategy — see [`registry`]; the rest is data and edit rules.

mod behavior;
mod item;
mod kind;
mod merge;
mod registry;
mod set;
mod value;

pub use behavior::{Expand, MarkBehavior, Overlap};
pub use item::Mark;
pub use kind::MarkKind;
pub use registry::{KindSpec, Registry, REGISTRY};
pub use set::MarkSet;
pub use value::{LinkStyle, LinkValue, MarkValue};
