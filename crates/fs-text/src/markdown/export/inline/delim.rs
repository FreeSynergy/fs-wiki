//! Choosing between `*` and `_` delimiters.
//!
//! Pattern: Strategy by source style — `_` is kept when the source used it and it still parses.

use crate::mark::{Mark, MarkValue};

/// `_`/`__` delimiters are kept when the source used them and they still parse (no word
/// character directly outside). With `star_follows` a `*` run stands between the span and the
/// next character, so that character does not count.
pub(super) fn delim(
    m: &Mark,
    chars: &[char],
    s: usize,
    e: usize,
    def: &'static str,
    under: &'static str,
    star_follows: bool,
) -> &'static str {
    if m.value == MarkValue::Name("_".into()) {
        let before = s.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let after = chars.get(e).copied();
        let word_after = !star_follows && after.is_some_and(char::is_alphanumeric);
        if !before.is_some_and(char::is_alphanumeric) && !word_after {
            return under;
        }
    }
    def
}
