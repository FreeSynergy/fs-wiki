//! Raw HTML blocks, for the admonition pre-pass.
//!
//! Pattern: Recogniser — says where an HTML block starts and ends (CommonMark 4.6) so that its
//! content is left alone.

/// Where a raw HTML block that cannot be interrupted ends.
pub(super) enum HtmlEnd {
    /// At the first line that contains this text (CommonMark HTML block types 1 to 5).
    Marker(&'static str),
    /// At the next blank line (types 6 and 7).
    Blank,
}

/// Does `line` open an HTML block? (CommonMark 4.6; type 7 is approximated by "a tag starts the
/// line" because the point is only to leave HTML content alone.)
pub(super) fn html_block_start(line: &str, at_block_start: bool) -> Option<HtmlEnd> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() >= 4 || !t.starts_with('<') {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let raw = |tag: &str| {
        lower
            .strip_prefix(tag)
            .is_some_and(|r| r.is_empty() || r.starts_with(['>', ' ', '\t']))
    };
    for (open, close) in [
        ("<pre", "</pre>"),
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<textarea", "</textarea>"),
    ] {
        if raw(open) {
            return Some(HtmlEnd::Marker(close));
        }
    }
    if t.starts_with("<!--") {
        return Some(HtmlEnd::Marker("-->"));
    }
    if t.starts_with("<?") {
        return Some(HtmlEnd::Marker("?>"));
    }
    if t.starts_with("<![CDATA[") {
        return Some(HtmlEnd::Marker("]]>"));
    }
    if t.starts_with("<!") && t[2..].starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Some(HtmlEnd::Marker(">"));
    }
    let tag = t[1..].strip_prefix('/').unwrap_or(&t[1..]);
    (at_block_start && tag.starts_with(|c: char| c.is_ascii_alphabetic())).then_some(HtmlEnd::Blank)
}
