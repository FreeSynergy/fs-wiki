//! Escaping literal text so it parses back as text.
//!
//! Pattern: Rule table — one `match` on the char decides whether a backslash is needed, using
//! its neighbours and whether it sits at a line start, in a table or in a link.

use super::ctx::Ctx;
use crate::markdown::HARD_BREAK;

pub(super) fn escape_into(out: &mut String, chars: &[char], p: usize, cx: Ctx, brackets: bool) {
    let c = chars[p];
    let prev = if p > 0 { Some(chars[p - 1]) } else { None };
    let next = chars.get(p + 1).copied();
    // text edges count as non-blank: a delimiter run may sit right next to them
    let ws = |o: Option<char>| o.map(|c| c.is_whitespace()).unwrap_or(false);
    let line_start =
        !cx.table && !cx.heading && (p == 0 || matches!(chars[p - 1], '\n' | HARD_BREAK));
    let next_blank = matches!(next, None | Some(' ') | Some('\n') | Some(HARD_BREAK));
    let esc = match c {
        HARD_BREAK => {
            out.push_str("\\\n");
            return;
        }
        '\t' => {
            out.push(' ');
            return;
        }
        '\\' => next
            .map(|n| n.is_ascii_punctuation() || n == '\n')
            .unwrap_or(true),
        '`' => true,
        '*' => !(ws(prev) && ws(next)),
        '_' => {
            !((ws(prev) && ws(next))
                || (prev.map(char::is_alphanumeric).unwrap_or(false)
                    && next.map(char::is_alphanumeric).unwrap_or(false)))
        }
        '[' => {
            brackets
                || next == Some('^')
                || next == Some('[')
                || (line_start && matches!(next, Some(' ' | 'x' | 'X' | '!')))
        }
        ']' => brackets || next == Some('('),
        '<' => matches!(next, Some(n) if n.is_alphanumeric() || n == '/' || n == '!' || n == '?'),
        '&' => {
            let rest: String = chars[p + 1..].iter().take(12).collect();
            rest.starts_with('#')
                || rest
                    .split(';')
                    .next()
                    .map(|w| {
                        rest.contains(';')
                            && !w.is_empty()
                            && w.chars().all(|c| c.is_ascii_alphanumeric())
                    })
                    .unwrap_or(false)
        }
        '~' => prev == Some('~') || next == Some('~'),
        '|' => cx.table,
        '#' if line_start => {
            let n = chars[p..].iter().take_while(|c| **c == '#').count();
            n <= 6
                && matches!(
                    chars.get(p + n).copied(),
                    None | Some(' ') | Some('\n') | Some(HARD_BREAK)
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
