//! [`Text`]: the editable text value used by blocks.
//!
//! Pattern: Facade over a Strategy — one small type with the API the crate uses; the storage
//! strategy is a type parameter (default: the rope), so tests and callers never name it.

use super::{DefaultStorage, TextStorage};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Range;

/// Editable text addressed by **char offsets** (not bytes). Serialises as a plain string.
#[derive(Clone, Default)]
pub struct Text<S: TextStorage = DefaultStorage> {
    store: S,
}

impl<S: TextStorage> Text<S> {
    /// Text holding `s`.
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self {
            store: S::from_text(s),
        }
    }
    /// Length in chars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.store.len_chars()
    }
    /// No chars at all?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Insert `s` at char offset `at` (clamped to the length).
    pub fn insert(&mut self, at: usize, s: &str) {
        self.store.insert_at(at, s);
    }
    /// Delete the char range (clamped).
    pub fn delete(&mut self, range: Range<usize>) {
        self.store.remove_range(range);
    }
    /// Copy a char range out as `String` (clamped).
    #[must_use]
    pub fn slice(&self, range: Range<usize>) -> String {
        self.store.slice_to_string(range)
    }
    /// Char at `i`, if any.
    #[must_use]
    pub fn char_at(&self, i: usize) -> Option<char> {
        self.store.char_at(i)
    }
    /// All chars in order.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.store.chars()
    }
}

impl<S: TextStorage> std::fmt::Display for Text<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.store.write_to(f)
    }
}
impl<S: TextStorage> std::fmt::Debug for Text<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}
impl<S: TextStorage> PartialEq for Text<S> {
    fn eq(&self, o: &Self) -> bool {
        self.store == o.store
    }
}
impl<S: TextStorage> Eq for Text<S> {}
impl<S: TextStorage> From<&str> for Text<S> {
    fn from(s: &str) -> Self {
        Text::new(s)
    }
}
impl<S: TextStorage> Serialize for Text<S> {
    fn serialize<T: Serializer>(&self, s: T) -> Result<T::Ok, T::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl<'de, S: TextStorage> Deserialize<'de> for Text<S> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Text::new(&String::deserialize(d)?))
    }
}
