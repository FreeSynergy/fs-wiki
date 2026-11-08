//! Words of a text and of a search term.
//!
//! Pattern: Strategy (pure functions) — a word is a run of letters and digits; matching ignores case
//! (Unicode lowercase). The last word of a search may match as a prefix. Offsets are UTF-8 bytes.

/// `text` in normalisation form C, as the store compares texts: `a` + U+0308 is `ä`.
pub(super) fn nfc(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    text.nfc().collect()
}

/// The byte ranges of the words of `text`.
pub(super) fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = vec![];
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                spans.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        spans.push((s, text.len()));
    }
    spans
}

/// The words of a search term, lowercased, in order.
pub(super) fn query_words(term: &str) -> Vec<String> {
    let term = nfc(term);
    word_spans(&term)
        .into_iter()
        .map(|(s, e)| term[s..e].to_lowercase())
        .collect()
}

/// What a search looks for.
pub(super) struct Needles {
    pub words: Vec<String>,
    /// The last word matches as a prefix.
    pub prefix: bool,
}

impl Needles {
    /// Whether `token` (any case) is the word `i` of the search.
    fn is(&self, i: usize, token: &str) -> bool {
        let token = token.to_lowercase();
        if self.prefix && i + 1 == self.words.len() {
            token.starts_with(&self.words[i])
        } else {
            token == self.words[i]
        }
    }

    /// Only the word `i`, with the prefix rule it has in the whole search.
    pub(super) fn only(&self, i: usize) -> Needles {
        Needles {
            words: vec![self.words[i].clone()],
            prefix: self.prefix && i + 1 == self.words.len(),
        }
    }

    /// The byte ranges in `text` of words that are any of the needles.
    pub(super) fn matches(&self, text: &str) -> Vec<(usize, usize)> {
        word_spans(text)
            .into_iter()
            .filter(|(s, e)| (0..self.words.len()).any(|i| self.is(i, &text[*s..*e])))
            .collect()
    }

    /// How many of the needles occur in `text`.
    pub(super) fn hits(&self, text: &str) -> usize {
        let tokens = word_spans(text);
        (0..self.words.len())
            .filter(|i| tokens.iter().any(|(s, e)| self.is(*i, &text[*s..*e])))
            .count()
    }
}
