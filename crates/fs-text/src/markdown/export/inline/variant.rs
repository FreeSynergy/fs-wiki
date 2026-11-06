//! Ways to write the same inline marks.
//!
//! Pattern: Strategy value — a [`Variant`] switches the writing rules that CommonMark's
//! delimiter matching is sensitive to; `verify` tries them until the text reads back the same.

use super::plan::Em;

/// How literal text is escaped.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(in crate::markdown::export) enum Form {
    /// Only what would be misread.
    #[default]
    Plain,
    /// Every ASCII punctuation char of the literal text: the form that cannot be misread, used
    /// when no other form reads back right.
    Everything,
}

/// Writing rules for runs of delimiter characters. The default is the plain, escaped form.
#[derive(Clone, Copy, Default, Debug)]
pub(in crate::markdown::export) struct Variant {
    /// A literal `*`/`_` right before an opener is not escaped: it joins the opening run
    /// (`___a_` is a literal `_` and two emphases), which keeps the run longer.
    pub lit_before_open: bool,
    /// A literal `*`/`_` right after a closer joins the closing run (`*a**` is an emphasis and
    /// a literal star), so the closer stays right-flanking only.
    pub lit_after_close: bool,
    /// An emphasis opened right behind a `*` keeps its `*` (no `_` swap).
    pub keep_star: bool,
    /// How literal text is escaped.
    pub form: Form,
}

impl Variant {
    /// All variants except the default, fewest deviations first.
    pub fn alternatives() -> impl Iterator<Item = Variant> {
        (1u8..8).map(|n| Variant {
            lit_before_open: n & 1 != 0,
            lit_after_close: n & 2 != 0,
            keep_star: n & 4 != 0,
            form: Form::Plain,
        })
    }

    /// The form that escapes everything.
    pub fn everything() -> Variant {
        Variant {
            form: Form::Everything,
            ..Variant::default()
        }
    }

    /// Is the literal `c` at `p` written without escape? `closed` is the last char of a closer
    /// written right before `c`, if nothing else came in between.
    pub(super) fn raw_delimiter_char(
        self,
        c: char,
        ems: &[Em],
        next_starts: std::ops::Range<usize>,
        ds: &[&str],
        closed: Option<(char, usize)>,
    ) -> bool {
        if !matches!(c, '*' | '_') {
            return false;
        }
        let before_open = next_starts.into_iter().any(|k| {
            let e = &ems[k];
            !e.atomic && e.e > e.s && ds[k].starts_with(c)
        });
        (self.lit_before_open && before_open)
            || (self.lit_after_close && closed.is_some_and(|(k, _)| k == c))
    }
}
