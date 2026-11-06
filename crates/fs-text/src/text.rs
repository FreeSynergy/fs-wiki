//! Rope text, addressed by **char offsets** (Unicode scalar values).

use ropey::Rope;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Range;

/// Editable text backed by a rope. Serialises as a plain string.
#[derive(Clone, Default)]
pub struct Text {
    rope: Rope,
}

impl Text {
    pub fn new(s: &str) -> Self {
        Self { rope: Rope::from_str(s) }
    }
    /// Length in chars.
    pub fn len(&self) -> usize {
        self.rope.len_chars()
    }
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }
    /// Insert `s` at char offset `at` (clamped to the length).
    pub fn insert(&mut self, at: usize, s: &str) {
        let at = at.min(self.len());
        self.rope.insert(at, s);
    }
    /// Delete the char range (clamped).
    pub fn delete(&mut self, range: Range<usize>) {
        let end = range.end.min(self.len());
        let start = range.start.min(end);
        if start < end {
            self.rope.remove(start..end);
        }
    }
    /// Copy a char range out as `String` (clamped).
    pub fn slice(&self, range: Range<usize>) -> String {
        let end = range.end.min(self.len());
        let start = range.start.min(end);
        self.rope.slice(start..end).to_string()
    }
    pub fn char_at(&self, i: usize) -> Option<char> {
        (i < self.len()).then(|| self.rope.char(i))
    }
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.rope.chars()
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.rope.chunks() {
            f.write_str(c)?;
        }
        Ok(())
    }
}
impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}
impl PartialEq for Text {
    fn eq(&self, o: &Self) -> bool {
        self.rope == o.rope
    }
}
impl Eq for Text {}
impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::new(s)
    }
}
impl Serialize for Text {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Text::new(&String::deserialize(d)?))
    }
}
