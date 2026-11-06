//! Adding, merging and cutting marks.
//!
//! Pattern: Strategy client — whether two marks merge is decided by the kind's
//! [`Overlap`] behaviour, not by this code.

use super::behavior::Overlap;
use super::item::Mark;
use super::kind::MarkKind;
use super::set::MarkSet;
use crate::id::{new_id, Uuid};
use std::cmp::Reverse;

impl MarkSet {
    /// Add a mark. Kinds with [`Overlap::Merge`] are merged with overlapping or touching
    /// marks of the same kind and value (the earliest id survives).
    pub fn add(&mut self, mut mark: Mark) -> Uuid {
        if mark.kind.behavior().overlap() == Overlap::Merge && !mark.is_point() {
            let mut i = 0;
            while i < self.marks.len() {
                let o = &self.marks[i];
                if o.kind == mark.kind
                    && o.value == mark.value
                    && o.start <= mark.end
                    && mark.start <= o.end
                    && !o.is_point()
                {
                    let o = self.marks.remove(i);
                    mark.start = mark.start.min(o.start);
                    mark.end = mark.end.max(o.end);
                    mark.id = o.id;
                } else {
                    i += 1;
                }
            }
        }
        let id = mark.id;
        let pos = self
            .marks
            .partition_point(|m| (m.start, Reverse(m.end)) <= (mark.start, Reverse(mark.end)));
        self.marks.insert(pos, mark);
        id
    }

    /// Remove `kind` from `start..end`: marks are cut or split around the range.
    pub fn unmark(&mut self, kind: MarkKind, start: usize, end: usize) {
        let mut out = Vec::new();
        for m in self.marks.drain(..) {
            if m.kind != kind || m.is_point() || m.end <= start || m.start >= end {
                out.push(m);
                continue;
            }
            if m.start < start {
                let mut l = m.clone();
                l.end = start;
                out.push(l);
            }
            if m.end > end {
                let mut r = m.clone();
                if m.start < start {
                    r.id = new_id();
                }
                r.start = end;
                out.push(r);
            }
        }
        *self = MarkSet::from_vec(out);
    }

    /// Merge overlapping / touching marks of [`Overlap::Merge`] kinds.
    pub fn normalize(&mut self) {
        let old = std::mem::take(&mut self.marks);
        for m in old {
            self.add(m);
        }
    }
}
