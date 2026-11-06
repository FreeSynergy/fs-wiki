//! The marks of one block: storage, queries and edit transforms.
//!
//! Pattern: Iterator + collection facade — a sorted `Vec` hidden behind [`MarkSet`]; merging
//! and splitting live in `merge.rs`.

use super::item::Mark;
use crate::id::Uuid;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;

/// The marks of one block, sorted by `(start, longest first)`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkSet {
    pub(super) marks: Vec<Mark>,
}

impl MarkSet {
    /// Empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Set from marks in any order.
    #[must_use]
    pub fn from_vec(mut marks: Vec<Mark>) -> Self {
        marks.sort_by_key(|m| (m.start, Reverse(m.end)));
        Self { marks }
    }
    /// Iterate in order.
    pub fn iter(&self) -> std::slice::Iter<'_, Mark> {
        self.marks.iter()
    }
    /// The marks as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[Mark] {
        &self.marks
    }
    /// Number of marks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.marks.len()
    }
    /// No marks?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
    /// Mark by id.
    #[must_use]
    pub fn get(&self, id: Uuid) -> Option<&Mark> {
        self.marks.iter().find(|m| m.id == id)
    }
    /// Mutable mark by id.
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Mark> {
        self.marks.iter_mut().find(|m| m.id == id)
    }
    /// Remove a mark by id.
    pub fn remove(&mut self, id: Uuid) -> Option<Mark> {
        let i = self.marks.iter().position(|m| m.id == id)?;
        Some(self.marks.remove(i))
    }
    /// Marks covering char `offset` (ranges: `start <= offset < end`; points: `== offset`).
    pub fn at(&self, offset: usize) -> impl Iterator<Item = &Mark> {
        self.marks.iter().filter(move |m| {
            (m.start <= offset && offset < m.end) || (m.is_point() && m.start == offset)
        })
    }

    /// Move all marks through an insertion.
    pub fn on_insert(&mut self, at: usize, len: usize) {
        for m in &mut self.marks {
            m.on_insert(at, len);
        }
        self.resort();
    }

    /// Move all marks through a deletion. Returns the marks that died with the deleted text.
    pub fn on_delete(&mut self, a: usize, b: usize) -> Vec<Mark> {
        let mut dead = Vec::new();
        let mut keep = Vec::with_capacity(self.marks.len());
        for mut m in self.marks.drain(..) {
            let orig = m.clone();
            if m.on_delete(a, b) {
                keep.push(m);
            } else {
                dead.push(orig);
            }
        }
        self.marks = keep;
        self.resort();
        dead
    }

    pub(super) fn resort(&mut self) {
        self.marks.sort_by_key(|m| (m.start, Reverse(m.end)));
    }

    /// Clamp ends to `len` (defensive; used after bulk imports).
    pub fn clamp(&mut self, len: usize) {
        for m in &mut self.marks {
            m.start = m.start.min(len);
            m.end = m.end.min(len);
        }
    }
}

impl<'a> IntoIterator for &'a MarkSet {
    type Item = &'a Mark;
    type IntoIter = std::slice::Iter<'a, Mark>;
    fn into_iter(self) -> Self::IntoIter {
        self.marks.iter()
    }
}
