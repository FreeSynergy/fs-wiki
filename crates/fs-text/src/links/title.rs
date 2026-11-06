//! Links inside a callout title.
//!
//! Pattern: Scanner — a callout keeps its title as a plain string (`> [!note] See [x](a/b.md)`),
//! so its links are not marks. The scan finds `](dest)` and `[[dest|label]]` and lets the
//! [`LinkRewriter`] map the destinations.

use super::rewriter::LinkRewriter;

/// `title` with the destinations of its inline and wiki links rewritten; `None` if none changed.
pub(super) fn rewrite_title(title: &str, rewriter: &dyn LinkRewriter) -> Option<String> {
    let mut out = String::with_capacity(title.len());
    let mut rest = title;
    let mut changed = false;
    while let Some(at) = rest.find(['(', '[']) {
        let (head, tail) = rest.split_at(at);
        out.push_str(head);
        let (open, close, bracket) = if tail.starts_with("[[") {
            ("[[", "]]", true)
        } else if tail.starts_with('(') && out.ends_with(']') {
            ("(", ")", false)
        } else {
            out.push_str(&tail[..1]);
            rest = &tail[1..];
            continue;
        };
        let inner_at = open.len();
        let Some(end) = tail[inner_at..].find(close) else {
            out.push_str(tail);
            rest = "";
            break;
        };
        let inner = &tail[inner_at..inner_at + end];
        // the destination ends at a space (a link title follows) or `|` (a wiki label)
        let cut = inner
            .find(if bracket { '|' } else { ' ' })
            .unwrap_or(inner.len());
        let (dest, after) = inner.split_at(cut);
        let plain = dest.trim_start_matches('<').trim_end_matches('>');
        match rewriter.rewrite(plain) {
            Some(new) => {
                changed = true;
                let wrapped = if dest.starts_with('<') {
                    format!("<{new}>")
                } else {
                    new
                };
                for part in [open, &wrapped, after, close] {
                    out.push_str(part);
                }
            }
            None => out.push_str(&tail[..inner_at + end + close.len()]),
        }
        rest = &tail[inner_at + end + close.len()..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::MovePrefix;

    #[test]
    fn rewrites_inline_and_wiki_destinations() {
        let r = MovePrefix::new("t", "u");
        assert_eq!(
            rewrite_title("See [l](t/p.md \"x\") and [[t/q|Q]] (t)", &r).as_deref(),
            Some("See [l](u/p.md \"x\") and [[u/q|Q]] (t)")
        );
        assert_eq!(rewrite_title("no links (t/p.md)", &r), None);
    }
}
