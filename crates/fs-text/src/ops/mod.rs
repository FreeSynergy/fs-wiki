//! Operations on documents (ADR 0042 §10).
//!
//! Pattern: Command — every change is an [`Op`] value that applies itself and returns its
//! exact inverse; undo/redo and the activity log are built on that.

mod apply;
mod blocks;
mod command;
mod edit;
mod error;
mod marks;
mod op;

pub use command::Command;
pub use error::ApplyError;
pub use op::Op;
