//! The storage interface behind [`crate::Text`].
//!
//! Pattern: Strategy (interface) — everything in the crate edits text through this trait
//! (char offsets, Unicode scalar values), never through a concrete rope type.

use std::ops::Range;

/// A sequence of chars that can be edited by char offset.
///
/// Implementations must be cheap to clone (or at least correct) and clamp out-of-range
/// offsets instead of panicking.
pub trait TextStorage: Clone + Default + PartialEq {
    /// Storage holding `s`.
    fn from_text(s: &str) -> Self;
    /// Length in chars.
    fn len_chars(&self) -> usize;
    /// Insert `s` at char offset `at` (clamped to the length).
    fn insert_at(&mut self, at: usize, s: &str);
    /// Remove the char range (clamped).
    fn remove_range(&mut self, range: Range<usize>);
    /// Copy a char range out as `String` (clamped).
    fn slice_to_string(&self, range: Range<usize>) -> String;
    /// Char at `i`, if any.
    fn char_at(&self, i: usize) -> Option<char>;
    /// All chars in order.
    fn chars(&self) -> Box<dyn Iterator<Item = char> + '_>;
    /// Write the whole text to `out`.
    ///
    /// # Errors
    /// Propagates errors of the formatter.
    fn write_to(&self, out: &mut dyn std::fmt::Write) -> std::fmt::Result;
}
