//! Escaping literal text so it parses back as text.
//!
//! Pattern: Rule table — one `match` on the char decides whether a backslash is needed, using
//! its neighbours and whether it sits at a line start, in a table or in a link.

use super::ctx::Ctx;
use crate::markdown::HARD_BREAK;

/// What is known about the whole window being written (computed once per sweep).
pub(super) struct Facts {
    /// Number of `~` in the window.
    tildes: usize,
    /// Is there a `}` in the window (a heading `{#id}` could form)?
    braces: bool,
}

impl Facts {
    pub fn of(chars: &[char]) -> Self {
        Self {
            tildes: chars.iter().filter(|c| **c == '~').count(),
            braces: chars.contains(&'}'),
        }
    }
}

/// How one literal char is written.
pub(super) struct Esc<'a> {
    pub cx: Ctx,
    /// Inside a link or image text.
    pub in_link: bool,
    /// A delimiter character left unescaped on purpose so it joins the delimiter run next to it.
    pub raw: bool,
    /// Escape every ASCII punctuation char (the form that cannot be misread).
    pub all: bool,
    pub facts: &'a Facts,
}

/// Is the text from `p` on a link reference definition label (`[a]: b`)?
fn definition_label(chars: &[char], p: usize) -> bool {
    let rest = &chars[p + 1..];
    rest.iter()
        .position(|c| *c == ']')
        .is_some_and(|i| rest.get(i + 1) == Some(&':'))
}

/// Write the literal char `chars[p]` so that it reads back as text.
pub(super) fn escape_into(out: &mut String, chars: &[char], p: usize, how: &Esc) {
    let (cx, brackets, raw) = (how.cx, how.in_link, how.raw);
    if raw {
        out.push(chars[p]);
        return;
    }
    let c = chars[p];
    let prev = if p > 0 { Some(chars[p - 1]) } else { None };
    let next = chars.get(p + 1).copied();
    // text edges count as non-blank: a delimiter run may sit right next to them
    let ws = |o: Option<char>| o.is_some_and(char::is_whitespace);
    let line_start =
        !cx.table && !cx.heading && (p == 0 || matches!(chars[p - 1], '\n' | HARD_BREAK));
    let next_blank = matches!(next, None | Some(' ' | '\n' | HARD_BREAK));
    let esc = match c {
        HARD_BREAK => {
            out.push_str("\\\n");
            return;
        }
        '\t' => {
            // a tab at the edge of a line would be dropped or read as indentation
            let edge = p == 0
                || matches!(chars[p - 1], '\n' | HARD_BREAK)
                || matches!(next, None | Some('\n' | HARD_BREAK));
            out.push_str(if cx.table {
                " "
            } else if edge {
                "&#9;"
            } else {
                "\t"
            });
            return;
        }
        _ if how.all && c.is_ascii_punctuation() => true,
        '\\' => next.is_none_or(|n| n.is_ascii_punctuation() || n == '\n'),
        '`' => true,
        '*' => !(ws(prev) && ws(next)),
        '_' => {
            !((ws(prev) && ws(next))
                || (prev.is_some_and(char::is_alphanumeric)
                    && next.is_some_and(char::is_alphanumeric)))
        }
        '[' => {
            brackets
                || (line_start && definition_label(chars, p))
                || next == Some('^')
                || next == Some('[')
                || (line_start && matches!(next, Some(' ' | 'x' | 'X' | '!')))
        }
        ']' => brackets || next == Some('('),
        '<' => matches!(next, Some(n) if n.is_alphanumeric() || n == '/' || n == '!' || n == '?'),
        '&' => {
            let rest: String = chars[p + 1..].iter().take(12).collect();
            rest.starts_with('#')
                || rest.split(';').next().is_some_and(|w| {
                    rest.contains(';')
                        && !w.is_empty()
                        && w.chars().all(|c| c.is_ascii_alphanumeric())
                })
        }
        '~' => how.facts.tildes > 1,
        '!' => next == Some('['),
        '{' => cx.heading && how.facts.braces,
        '|' => cx.table,
        '#' if line_start => {
            let n = chars[p..].iter().take_while(|c| **c == '#').count();
            n <= 6
                && matches!(
                    chars.get(p + n).copied(),
                    None | Some(' ' | '\n' | HARD_BREAK)
                )
        }
        '>' if line_start => true,
        '+' | '-' if line_start => {
            next_blank
                || chars[p..]
                    .iter()
                    .take_while(|c| **c != '\n')
                    .all(|c| *c == '-' || *c == ' ')
        }
        '=' if line_start => chars[p..]
            .iter()
            .take_while(|c| **c != '\n')
            .all(|c| *c == '='),
        '.' | ')' if !cx.table && !cx.heading => {
            // ordered list marker at line start: digits then . or ) then blank
            let mut i = p;
            while i > 0 && chars[i - 1].is_ascii_digit() {
                i -= 1;
            }
            let digits = p - i;
            let at_start = i == 0 || matches!(chars[i - 1], '\n' | HARD_BREAK);
            digits > 0 && digits <= 9 && at_start && next_blank
        }
        _ => false,
    };
    if esc {
        out.push('\\');
    }
    out.push(c);
}
