//! Links inside a callout title.
//!
//! Pattern: Scanner — a callout keeps its title as Markdown as written (`> [!note] See
//! [x](a/b.md)`), not as marks, so a link in it is found here: `[text](dest)` / `![alt](dest)`
//! (Markdown, relative to the page) and `[[target|label]]` (a wiki link, from the root). The same
//! rule as for marks then decides what a move changes (`relink.rs`) and what the link index holds.

use fs_text::{Block, Container, LinkStyle, LinkValue};

/// A destination in a title: byte range and how it was written.
pub(super) struct TitleLink {
    pub start: usize,
    pub end: usize,
    pub style: LinkStyle,
}

/// The links of `title`, left to right.
pub(super) fn title_links(title: &str) -> Vec<TitleLink> {
    let mut found = vec![];
    let mut at = 0;
    while at < title.len() {
        let rest = &title[at..];
        if let Some(inner) = rest.strip_prefix("[[") {
            let len = inner.find(['|', ']']).unwrap_or(inner.len());
            found.push(TitleLink {
                start: at + 2,
                end: at + 2 + len,
                style: LinkStyle::Wiki,
            });
            at += 2 + len;
        } else if let Some(inner) = rest.strip_prefix("](") {
            let inner_trim = inner.strip_prefix('<');
            let skip = usize::from(inner_trim.is_some());
            let body = inner_trim.unwrap_or(inner);
            let len = body
                .find(|c: char| c.is_whitespace() || c == ')' || c == '>')
                .unwrap_or(body.len());
            found.push(TitleLink {
                start: at + 2 + skip,
                end: at + 2 + skip + len,
                style: LinkStyle::Inline,
            });
            at += 2 + skip + len;
        } else {
            at += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    found
}

/// The titles of the callouts `block` stands in.
pub(super) fn titles_of(block: &Block) -> impl Iterator<Item = &str> {
    block.attrs.containers.iter().filter_map(|c| match c {
        Container::Callout { title, .. } => Some(title.as_str()),
        _ => None,
    })
}

/// The links of the callout titles of `block`, as link values.
pub(super) fn links_in_titles(block: &Block) -> Vec<LinkValue> {
    titles_of(block)
        .flat_map(|t| {
            title_links(t)
                .into_iter()
                .map(|l| LinkValue::new(&t[l.start..l.end], l.style))
        })
        .collect()
}
