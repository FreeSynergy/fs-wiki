//! Ways to write the same inline marks.
//!
//! Pattern: Strategy value — a [`Variant`] switches the writing rules that CommonMark's
//! delimiter matching is sensitive to; `verify` tries them until the text reads back the same.

use super::plan::Em;

/// Writing rules for runs of delimiter characters. The default is the plain, escaped form.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Variant {
    /// A literal `*`/`_` right before an opener is not escaped: it joins the opening run
    /// (`___a_` is a literal `_` and two emphases), which keeps the run longer.
    pub lit_before_open: bool,
    /// A literal `*`/`_` right after a closer joins the closing run (`*a**` is an emphasis and
    /// a literal star), so the closer stays right-flanking only.
    pub lit_after_close: bool,
    /// An emphasis opened right behind a `*` keeps its `*` (no `_` swap).
    pub keep_star: bool,
}

impl Variant {
    /// All variants except the default, fewest deviations first.
    pub fn alternatives() -> impl Iterator<Item = Variant> {
        (1u8..8).map(|n| Variant {
            lit_before_open: n & 1 != 0,
            lit_after_close: n & 2 != 0,
            keep_star: n & 4 != 0,
        })
    }

    /// Is the literal `c` at `p` written without escape? `closed` is the last char of a closer
    /// written right before `c`, if nothing else came in between.
    pub fn raw_delimiter_char(
        self,
        c: char,
        p: usize,
        ems: &[Em],
        ds: &[&str],
        closed: Option<(char, usize)>,
    ) -> bool {
        if !matches!(c, '*' | '_') {
            return false;
        }
        let before_open = ems
            .iter()
            .zip(ds)
            .any(|(e, d)| e.s == p + 1 && !e.atomic && e.e > e.s && d.starts_with(c));
        (self.lit_before_open && before_open)
            || (self.lit_after_close && closed.is_some_and(|(k, _)| k == c))
    }
}
