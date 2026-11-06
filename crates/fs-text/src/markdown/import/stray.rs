//! Definitions the parser skipped: a second definition of a label.
//!
//! Pattern: Recogniser over the gaps — `pulldown-cmark` keeps the first definition of a label and
//! drops the others without an event, so they are in no block and would live only in a kept
//! source text. Every definition-looking run of lines that no block covers becomes an
//! HTML-kind block marked `unparsed`, like the first definitions (`refdef`).

use super::state::Imp;
use crate::doc::{Block, BlockKind};
use crate::id::new_id;

/// `[label]: …` at the start of a line (up to three spaces of indent).
fn definition_start(line: &str) -> bool {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return false;
    }
    let Some(rest) = t.strip_prefix('[') else {
        return false;
    };
    rest.find(']').is_some_and(|i| {
        i > 0
            && rest[i + 1..]
                .strip_prefix(':')
                .is_some_and(|d| !d.trim().is_empty())
    })
}

impl Imp<'_> {
    /// Add the stray definitions as blocks, in source order (`self.spans` stays parallel).
    pub(super) fn add_stray_definitions(&mut self, blocks: &mut Vec<Block>) {
        let Some(spans) = self.spans.iter().copied().collect::<Option<Vec<_>>>() else {
            return;
        };
        let lines: Vec<&str> = self.src.split('\n').collect();
        let mut cov = vec![false; lines.len()];
        for s in &spans {
            let last = s.1.min(lines.len().saturating_sub(1));
            cov[s.0.min(last)..=last].fill(true);
        }
        let covered = |l: usize| cov[l];
        let mut found: Vec<(usize, usize, String)> = Vec::new();
        let mut l = 0;
        while l < lines.len() {
            if !covered(l) && definition_start(lines[l]) {
                let mut e = l;
                while e + 1 < lines.len() && !covered(e + 1) && !lines[e + 1].trim().is_empty() {
                    e += 1;
                }
                found.push((l, e, lines[l..=e].join("\n")));
                l = e + 1;
            } else {
                l += 1;
            }
        }
        if found.is_empty() {
            return;
        }
        let mut merged: Vec<(Block, Option<(usize, usize)>)> = Vec::new();
        let old = std::mem::take(blocks);
        let mut old = old.into_iter().zip(spans).peekable();
        for (first, last, text) in found {
            while old.peek().is_some_and(|(_, s)| s.0 < first) {
                let (b, s) = old.next().expect("peeked");
                merged.push((b, Some(s)));
            }
            let mut b = Block::new(BlockKind::Html, &text);
            b.id = new_id();
            b.attrs.extra.insert("unparsed".into(), "1".into());
            merged.push((b, Some((first, last))));
        }
        merged.extend(old.map(|(b, s)| (b, Some(s))));
        (*blocks, self.spans) = merged.into_iter().unzip();
    }
}
