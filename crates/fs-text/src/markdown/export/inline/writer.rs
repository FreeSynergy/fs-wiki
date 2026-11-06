//! [`MarkWriter`] for every Markdown syntax: opening and closing text of a mark.
//!
//! Pattern: Strategy — [`MdSyntax`] is the strategy selector (from the registry in
//! `markdown::syntax`); inline links write `[text](dest "title")` on close.

use crate::mark::{Mark, MarkValue};
use crate::markdown::{MarkWriter, MdSyntax};

impl MarkWriter for MdSyntax {
    fn open(&self, d: &str) -> String {
        match self {
            MdSyntax::Delim(o, _) => {
                if d.is_empty() {
                    (*o).to_string()
                } else {
                    d.to_string()
                }
            }
            MdSyntax::Link | MdSyntax::Layer => "[".into(),
            MdSyntax::Image => "![".into(),
            MdSyntax::Raw | MdSyntax::FootnoteRef => String::new(),
        }
    }

    fn close(&self, m: &Mark, d: &str) -> String {
        match self {
            MdSyntax::Delim(_, c) => {
                if d.is_empty() {
                    (*c).to_string()
                } else {
                    d.to_string()
                }
            }
            MdSyntax::Link | MdSyntax::Image => match m.link() {
                Some(l) => format!(
                    "]({}{})",
                    dest(&l.path),
                    l.title.as_deref().map(title).unwrap_or_default()
                ),
                None => "]()".into(),
            },
            MdSyntax::Layer => match &m.value {
                MarkValue::Name(n) => format!("]{{#{n}}}"),
                _ => "]".into(),
            },
            MdSyntax::Raw | MdSyntax::FootnoteRef => String::new(),
        }
    }
}

/// `&` that starts something like a character reference (`&copy;`, `&#9;`).
fn starts_reference(rest: &str) -> bool {
    let body: String = rest
        .chars()
        .skip(1)
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '#')
        .collect();
    !body.is_empty() && rest[1 + body.len()..].starts_with(';')
}

/// Escapes that keep `s` literal inside a link destination or title: a backslash before
/// punctuation (or at the end) is doubled, an `&` that would start a character reference is
/// written `&amp;`, and `extra` chars get a backslash.
fn literal(s: &str, extra: &[char]) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        let next = s[i + c.len_utf8()..].chars().next();
        match c {
            '\\' if next.is_none_or(|n| n.is_ascii_punctuation()) => out.push_str("\\\\"),
            '&' if starts_reference(&s[i..]) => out.push_str("&amp;"),
            c if extra.contains(&c) => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

fn dest(d: &str) -> String {
    let opens = d.matches('(').count();
    let closes = d.matches(')').count();
    let plain = !d.is_empty()
        && opens == closes
        && !d.starts_with('<')
        && !d.contains(|c: char| c.is_whitespace() || c.is_control());
    if plain {
        literal(d, &[])
    } else {
        format!("<{}>", literal(d, &['<', '>']))
    }
}

fn title(t: &str) -> String {
    format!(" \"{}\"", literal(t, &['"']))
}
