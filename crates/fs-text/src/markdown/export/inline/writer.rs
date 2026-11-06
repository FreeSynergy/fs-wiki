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
                    l.title
                        .as_ref()
                        .map(|t| format!(" \"{}\"", t.replace('"', "\\\"")))
                        .unwrap_or_default()
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

fn dest(d: &str) -> String {
    let opens = d.matches('(').count();
    let closes = d.matches(')').count();
    if d.is_empty() || d.contains(char::is_whitespace) || opens != closes {
        format!("<{d}>")
    } else {
        d.to_string()
    }
}
