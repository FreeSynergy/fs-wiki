//! Snippets: a piece of a text around its matches.
//!
//! Pattern: Builder (pure function) — at most `MAX_SNIPPET_BYTES`, cut on character boundaries, never
//! through a highlight; a highlight is a whole matching word, in bytes of the snippet.

use crate::limits::{MAX_HIGHLIGHTS, MAX_SNIPPET_BYTES};
use crate::search::{Highlight, Snippet};

/// Bytes kept before the first match.
const LEAD: usize = 80;

fn floor(text: &str, mut i: usize) -> usize {
    i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The snippet of `text` (field `field`) around `matches` (sorted byte ranges), if there are any.
pub(super) fn snippet(
    field: &'static str,
    text: &str,
    matches: &[(usize, usize)],
) -> Option<Snippet> {
    let first = matches.first()?;
    let from = floor(text, first.0.saturating_sub(LEAD));
    let mut to = floor(text, from + MAX_SNIPPET_BYTES);
    // Do not end inside a match: end where that match starts.
    if let Some((s, _)) = matches.iter().find(|(s, e)| *s < to && *e > to) {
        to = *s;
    }
    let highlights: Vec<Highlight> = matches
        .iter()
        .filter(|(s, e)| *s >= from && *e <= to)
        .take(MAX_HIGHLIGHTS)
        .map(|(s, e)| Highlight {
            start: u32::try_from(s - from).unwrap_or(u32::MAX),
            end: u32::try_from(e - from).unwrap_or(u32::MAX),
        })
        .collect();
    Some(Snippet {
        field,
        text: text[from..to].to_string(),
        highlights,
        cut_before: from > 0,
        cut_after: to < text.len(),
    })
}
