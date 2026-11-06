//! The command interface.
//!
//! Pattern: Command (interface) — history and activity logs depend on this trait, not on
//! [`crate::Op`], so another command set can use the same machinery.

use super::error::ApplyError;

/// A reversible change to a `Target`.
pub trait Command: Sized {
    /// What the command changes.
    type Target;
    /// Apply the command and return its exact inverse.
    ///
    /// # Errors
    /// [`ApplyError`] if the command does not fit the target; the target is then unchanged.
    fn apply(&self, target: &mut Self::Target) -> Result<Self, ApplyError>;
}
