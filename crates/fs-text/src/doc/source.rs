//! The source text a block was read from (ADR 0030 keeps originals).
//!
//! Pattern: Memento — a [`Source`] remembers the exact text of a block (with the blank lines
//! before it and the file's line ending) together with a fingerprint of what the block looked
//! like when it was read. An exporter may write the text back verbatim, but only while the
//! fingerprint and the neighbour still match: a stale source is never used.
//!
//! PROVISIONAL (the owner decides the final rule): kept in `BlockAttrs::source`, so it is part of
//! the stored form, travels with `Op::SetAttrs` and is ignored by [`Document::same_content`].

use super::block::Block;
use crate::id::Uuid;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Exact source text of a block (or of a group of blocks that share source lines).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The text: blank lines before the block, the block's own lines, line endings as read.
    /// For the last block of a file this includes everything up to the end of the file.
    pub text: String,
    /// The block in front of this one when the text was read (`None` = first block).
    pub prev: Option<Uuid>,
    /// Fingerprint of the block's content and marks when the text was read.
    pub hash: String,
    /// Blocks that share source lines with this one (their text is part of `text`, they have no
    /// source of their own); all must be unchanged for `text` to be used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rest: Vec<(Uuid, String)>,
}

/// 128-bit FNV-1a as a [`std::fmt::Write`] sink, so `Debug` output is hashed without a copy.
struct Fnv(u128);

impl std::fmt::Write for Fnv {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for b in s.bytes() {
            self.0 ^= u128::from(b);
            self.0 = self
                .0
                .wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013B);
        }
        Ok(())
    }
}

impl Block {
    /// Fingerprint of what the block says: text, marks (kind, range, value; not their ids),
    /// kind, containers and the other attributes (not the source itself).
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let mut h = Fnv(0x6c62_272e_07bb_0142_62b8_2175_6295_c58d);
        let a = &self.attrs;
        let _ = write!(
            h,
            "{:?}\u{1}{:?}\u{1}{:?}\u{1}{:?}\u{1}{:?}\u{1}{}\u{1}",
            a.kind, a.containers, a.block_ref, a.heading_attrs, a.extra, self.text
        );
        for m in &self.marks {
            let _ = write!(
                h,
                "{:?}\u{2}{}\u{2}{}\u{2}{:?}\u{1}",
                m.kind, m.start, m.end, m.value
            );
        }
        format!("{:032x}", h.0)
    }
}

/// How many blocks starting at `at` the source of `blocks[at]` stands for, if it may be used:
/// the block and the ones sharing its lines are unchanged and the block in front is the same.
#[must_use]
pub fn usable_source(blocks: &[Block], at: usize) -> Option<(&Source, usize)> {
    let b = blocks.get(at)?;
    let s = b.attrs.source.as_ref()?;
    let prev = at.checked_sub(1).map(|i| blocks[i].id);
    if s.prev != prev || s.hash != b.fingerprint() {
        return None;
    }
    for (k, (id, hash)) in s.rest.iter().enumerate() {
        let o = blocks.get(at + 1 + k)?;
        if o.id != *id || o.fingerprint() != *hash {
            return None;
        }
    }
    Some((s, 1 + s.rest.len()))
}
