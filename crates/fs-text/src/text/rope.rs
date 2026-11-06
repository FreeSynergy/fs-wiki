//! `ropey`-backed [`TextStorage`].
//!
//! Pattern: Adapter — wraps the foreign `ropey::Rope` and implements our storage trait.

use super::storage::TextStorage;
use ropey::Rope;
use std::ops::Range;

/// Text stored in a `ropey` rope.
#[derive(Clone, Default, PartialEq)]
pub struct RopeStorage {
    rope: Rope,
}

fn clamp(range: Range<usize>, len: usize) -> Range<usize> {
    let end = range.end.min(len);
    range.start.min(end)..end
}

impl TextStorage for RopeStorage {
    fn from_text(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
        }
    }
    fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }
    fn insert_at(&mut self, at: usize, s: &str) {
        let at = at.min(self.len_chars());
        self.rope.insert(at, s);
    }
    fn remove_range(&mut self, range: Range<usize>) {
        let r = clamp(range, self.len_chars());
        if r.start < r.end {
            self.rope.remove(r);
        }
    }
    fn slice_to_string(&self, range: Range<usize>) -> String {
        let r = clamp(range, self.len_chars());
        self.rope.slice(r).to_string()
    }
    fn char_at(&self, i: usize) -> Option<char> {
        (i < self.len_chars()).then(|| self.rope.char(i))
    }
    fn chars(&self) -> Box<dyn Iterator<Item = char> + '_> {
        Box::new(self.rope.chars())
    }
    fn write_to(&self, out: &mut dyn std::fmt::Write) -> std::fmt::Result {
        for chunk in self.rope.chunks() {
            out.write_str(chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_are_clamped() {
        let mut s = RopeStorage::from_text("héllo");
        s.insert_at(99, "!");
        s.remove_range(1..3);
        assert_eq!(s.slice_to_string(0..99), "hlo!");
        assert_eq!(s.char_at(1), Some('l'));
        assert_eq!(s.char_at(9), None);
    }
}
