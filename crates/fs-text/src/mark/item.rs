//! One mark and how it moves through edits.
//!
//! * `start == end`  -> **point**: something is *inserted* here (variable, footnote ref).
//! * `start <  end`  -> **range**: the text is *named* / *replaced* / formatted.
//!
//! Pattern: Strategy client — boundary decisions come from the mark's [`Expand`] rule, which
//! the kind's [`crate::MarkBehavior`] provides.

use super::behavior::Expand;
use super::kind::MarkKind;
use super::value::{LinkValue, MarkValue};
use crate::id::{new_id, Uuid};
use serde::{Deserialize, Serialize};

/// A stand-off mark: lives next to the text, never inside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    /// Identity.
    pub id: Uuid,
    /// Meaning.
    pub kind: MarkKind,
    /// Payload.
    pub value: MarkValue,
    /// Char offsets into the block text. `start == end` = point.
    pub start: usize,
    /// End offset (exclusive).
    pub end: usize,
    /// Boundary behaviour (defaults to the kind's).
    pub expand: Expand,
}

impl Mark {
    /// New range mark with the kind's default expansion.
    #[must_use]
    pub fn new(kind: MarkKind, start: usize, end: usize, value: MarkValue) -> Self {
        Self {
            id: new_id(),
            kind,
            value,
            start: start.min(end),
            end,
            expand: kind.behavior().expand(),
        }
    }
    /// New point mark.
    #[must_use]
    pub fn point(kind: MarkKind, at: usize, value: MarkValue) -> Self {
        Self::new(kind, at, at, value)
    }
    /// Is it a point?
    #[must_use]
    pub fn is_point(&self) -> bool {
        self.start == self.end
    }
    /// The link value, for link / image marks.
    #[must_use]
    pub fn link(&self) -> Option<&LinkValue> {
        match &self.value {
            MarkValue::Link(l) => Some(l),
            _ => None,
        }
    }

    /// Move through an insertion of `len` chars at `at`.
    pub fn on_insert(&mut self, at: usize, len: usize) {
        if self.is_point() {
            // A point stays put (typed text lands after it) iff expand contains `before`.
            if self.start > at || (self.start == at && !self.expand.before()) {
                self.start += len;
            }
            self.end = self.start;
            return;
        }
        if self.start > at || (self.start == at && !self.expand.before()) {
            self.start += len;
        }
        if self.end > at || (self.end == at && self.expand.after()) {
            self.end += len;
        }
    }

    /// Move through a deletion of `a..b`. Returns `false` if the mark died: its whole text
    /// (or the point's surrounding text) is gone and it takes the mark along (ADR 0042 §6).
    pub fn on_delete(&mut self, a: usize, b: usize) -> bool {
        if a >= b {
            return true;
        }
        if self.is_point() {
            if self.start > a && self.start < b {
                return false;
            }
        } else if self.start >= a && self.end <= b {
            return false;
        }
        let m = |o: usize| {
            if o <= a {
                o
            } else if o >= b {
                o - (b - a)
            } else {
                a
            }
        };
        self.start = m(self.start);
        self.end = m(self.end);
        true
    }
}
