//! Anchors: a position that survives edits (ADR 0042, wiki-ideas Part 2).
//!
//! Internally a char offset; the API is the only thing callers touch, so it can later be
//! swapped for CRDT ids.
//!
//! Pattern: Newtype / value object — `Anchor` is a small immutable value with the transform
//! rules attached, so callers never do offset arithmetic themselves.

use serde::{Deserialize, Serialize};

/// Which side an anchor sticks to when text is inserted exactly at its offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bias {
    /// Stay put; inserted text ends up after the anchor.
    Left,
    /// Move along; inserted text ends up before the anchor.
    Right,
}

/// A char offset plus the side it sticks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// Char offset.
    pub offset: usize,
    /// Side it sticks to on insertion at `offset`.
    pub bias: Bias,
}

impl Anchor {
    /// Anchor at `offset` with the given bias.
    #[must_use]
    pub fn new(offset: usize, bias: Bias) -> Self {
        Self { offset, bias }
    }
    #[must_use]
    /// Position after inserting `len` chars at `at`.
    pub fn after_insert(self, at: usize, len: usize) -> Self {
        let moves = self.offset > at || (self.offset == at && self.bias == Bias::Right);
        Self {
            offset: if moves {
                self.offset + len
            } else {
                self.offset
            },
            bias: self.bias,
        }
    }
    #[must_use]
    /// Position after deleting `start..end`. Positions inside collapse to `start`.
    pub fn after_delete(self, start: usize, end: usize) -> Self {
        let o = self.offset;
        let n = if o <= start {
            o
        } else if o >= end {
            o - (end - start)
        } else {
            start
        };
        Self {
            offset: n,
            bias: self.bias,
        }
    }
}
