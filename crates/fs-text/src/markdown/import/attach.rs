//! Keeping the source text of every block.
//!
//! Pattern: Memento — after the blocks are built, the file is cut into pieces by the source
//! lines of the blocks: each block remembers its own lines plus the lines (blank lines, comment
//! separators) between it and the block before; the last one takes everything up to the end of
//! the file. Blocks that share a line (`- > a`) form one group that owns the lines together.
//! Written back in order, the pieces give the file byte for byte (ADR 0030 keeps originals).

use super::state::Imp;
use crate::doc::{Block, Source};

impl Imp<'_> {
    /// A block is "glued" when it stands on the line right after the block before it (no blank
    /// line, no comment between): tight lists, a fence under a paragraph. Written as the hint
    /// `glue` so the canonical writer can reproduce it.
    pub(super) fn settle_glue(&self, blocks: &mut [Block]) {
        for (i, b) in blocks.iter_mut().enumerate() {
            let glued = i > 0
                && matches!((self.spans[i - 1], self.spans[i]), (Some(p), Some(s)) if p.1 + 1 == s.0);
            if glued {
                b.attrs.extra.insert("glue".into(), "1".into());
            }
        }
    }

    /// Give the blocks their [`Source`]. `orig` is the file as read (line endings, BOM).
    pub(super) fn attach_sources(&self, blocks: &mut Vec<Block>, orig: &str) {
        if orig.is_empty() {
            return;
        }
        if blocks.is_empty() {
            // only blank lines: one empty paragraph keeps them
            let mut b = Block::paragraph("");
            b.attrs.source = Some(Source {
                text: orig.to_string(),
                prev: None,
                hash: b.fingerprint(),
                rest: Vec::new(),
            });
            blocks.push(b);
            return;
        }
        let spans: Option<Vec<(usize, usize)>> = self.spans.iter().copied().collect();
        let Some(spans) = spans.filter(|s| s.len() == blocks.len()) else {
            return;
        };
        let lines = split_lines(orig);
        let last_line = lines.len().saturating_sub(1);
        let hashes: Vec<String> = blocks.iter().map(Block::fingerprint).collect();
        let mut cursor = 0;
        let mut i = 0;
        while i < blocks.len() {
            let mut end = spans[i].1;
            let mut j = i + 1;
            while j < blocks.len() && spans[j].0 <= end {
                end = end.max(spans[j].1);
                j += 1;
            }
            if j == blocks.len() {
                end = last_line;
            }
            let end = end.min(last_line);
            let text: String = lines[cursor.min(end + 1).min(lines.len())..=end].concat();
            cursor = end + 1;
            let prev = i.checked_sub(1).map(|p| blocks[p].id);
            let rest = (i + 1..j)
                .map(|k| (blocks[k].id, hashes[k].clone()))
                .collect();
            blocks[i].attrs.source = Some(Source {
                text,
                prev,
                hash: hashes[i].clone(),
                rest,
            });
            i = j;
        }
    }
}

/// The lines of `text` with their line ends; a line ends with LF, CRLF or a lone CR.
fn split_lines(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let (mut lines, mut from) = (Vec::new(), 0);
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\n' => {
                lines.push(&text[from..=i]);
                from = i + 1;
            }
            b'\r' => {
                if b.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                lines.push(&text[from..=i]);
                from = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if from < b.len() {
        lines.push(&text[from..]);
    }
    lines
}
