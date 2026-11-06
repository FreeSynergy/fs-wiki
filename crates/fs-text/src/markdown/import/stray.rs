//! Definitions the parser skipped: a second definition of a label.
//!
//! Pattern: Recogniser over the gaps — `pulldown-cmark` keeps the first definition of a label and
//! drops the others without an event, so they are in no block and would live only in a kept
//! source text. Every definition-looking run of lines that no block covers becomes an
//! HTML-kind block marked `unparsed`, like the first definitions (`refdef`).

use super::state::Imp;
use crate::doc::{Block, BlockKind, Container};
use crate::id::new_id;

/// Strip the quote marks (`> `) in front of a line: how many and the rest.
fn unquote(line: &str) -> (usize, &str) {
    let (mut depth, mut rest) = (0, line);
    loop {
        let t = rest.trim_start_matches(' ');
        if rest.len() - t.len() > 3 {
            return (depth, rest);
        }
        match t.strip_prefix('>') {
            Some(r) => {
                depth += 1;
                rest = r.strip_prefix(' ').unwrap_or(r);
            }
            None => return (depth, rest),
        }
    }
}

/// `[label]: …` at the start of a line (up to three spaces of indent); the destination may be
/// on the next line (`more`).
fn definition_start(line: &str, more: bool) -> bool {
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
                .is_some_and(|d| !d.trim().is_empty() || more)
    })
}

/// Do the first `depth` containers all quote (quote or callout)?
fn quotes(containers: &[Container], depth: usize) -> bool {
    containers.len() >= depth
        && containers[..depth]
            .iter()
            .all(|c| matches!(c, Container::Quote { .. } | Container::Callout { .. }))
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
        // (first line, last line, quote depth, text without the quote marks)
        let mut found: Vec<(usize, usize, usize, String)> = Vec::new();
        let mut l = 0;
        while l < lines.len() {
            let (depth, first) = unquote(lines[l]);
            let more = lines
                .get(l + 1)
                .is_some_and(|n| !covered(l + 1) && !unquote(n).1.trim().is_empty());
            if !covered(l) && definition_start(first, more) {
                let mut e = l;
                while e + 1 < lines.len()
                    && !covered(e + 1)
                    && !unquote(lines[e + 1]).1.trim().is_empty()
                {
                    e += 1;
                }
                let text: Vec<&str> = lines[l..=e].iter().map(|x| unquote(x).1).collect();
                found.push((l, e, depth, text.join("\n")));
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
        for (first, last, depth, text) in found {
            while old.peek().is_some_and(|(_, s)| s.0 < first) {
                let (b, s) = old.next().expect("peeked");
                merged.push((b, Some(s)));
            }
            let mut b = Block::new(BlockKind::Html, &text);
            b.id = new_id();
            b.attrs.extra.insert("unparsed".into(), "1".into());
            // behind `> `: in the quote of the block before or after it
            let around = merged
                .last()
                .map(|(p, _)| p)
                .into_iter()
                .chain(old.peek().map(|(n, _)| n));
            b.attrs.containers = around
                .filter(|n| quotes(&n.attrs.containers, depth))
                .map(|n| n.attrs.containers[..depth].to_vec())
                .next()
                .unwrap_or_default();
            if b.attrs.containers.len() < depth {
                // no quote to put it in: keep the marks as text
                let marks = ">".repeat(depth - b.attrs.containers.len());
                b = Block::new(BlockKind::Html, &format!("{marks} {text}"));
                b.attrs.extra.insert("unparsed".into(), "1".into());
            }
            merged.push((b, Some((first, last))));
        }
        merged.extend(old.map(|(b, s)| (b, Some(s))));
        (*blocks, self.spans) = merged.into_iter().unzip();
    }
}
