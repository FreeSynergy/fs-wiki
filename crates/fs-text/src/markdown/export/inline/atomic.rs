//! Marks whose content is written verbatim: code spans, HTML, autolinks, wiki links; and
//! point marks (footnote references, variables).
//!
//! Pattern: Strategy by kind — one small writer per atomic kind.

use crate::mark::{LinkStyle, Mark, MarkKind, MarkValue};
use crate::markdown::{md_syntax, MarkWriter};

pub(super) fn point_str(m: &Mark) -> String {
    match &m.value {
        // a link or image without text: `[](u)`, `![](u)`
        MarkValue::Link(l) if l.style == LinkStyle::Inline => {
            let s = md_syntax(m.kind);
            format!("{}{}", s.open(""), s.close(m, ""))
        }
        MarkValue::Link(l) if l.style == LinkStyle::Wiki => format!("[[{}]]", l.path),
        MarkValue::Link(l) if l.style == LinkStyle::Embed => format!("![[{}]]", l.path),
        MarkValue::Name(l) if m.kind == MarkKind::FootnoteRef => format!("[^{l}]"),
        MarkValue::Variable { name, .. } => format!("{{{{{name}}}}}"),
        _ => String::new(),
    }
}

fn code_span(content: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in content.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let f = "`".repeat(longest + 1);
    let pad = content.starts_with('`')
        || content.ends_with('`')
        || (content.starts_with(' ') && content.ends_with(' ') && !content.trim().is_empty());
    let content = content.replace('\n', " ");
    if pad {
        format!("{f} {content} {f}")
    } else {
        format!("{f}{content}{f}")
    }
}

pub(super) fn atomic_str(m: &Mark, content: &str) -> String {
    match m.kind {
        MarkKind::Code => code_span(content),
        MarkKind::Html => content.to_string(),
        _ => {
            let l = m.link().expect("link value");
            match l.style {
                LinkStyle::Autolink => format!("<{content}>"),
                LinkStyle::Wiki | LinkStyle::Embed => {
                    let bang = if l.style == LinkStyle::Embed { "!" } else { "" };
                    if content == l.path {
                        format!("{bang}[[{}]]", l.path)
                    } else {
                        format!("{bang}[[{}|{}]]", l.path, content)
                    }
                }
                LinkStyle::Inline => unreachable!("inline links are not atomic"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_spans_grow_and_pad() {
        assert_eq!(code_span("a"), "`a`");
        assert_eq!(code_span("a`b"), "``a`b``");
        assert_eq!(code_span("`a"), "`` `a ``");
    }
}
