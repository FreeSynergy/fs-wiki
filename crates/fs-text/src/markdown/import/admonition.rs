//! mkdocs admonitions (`!!! note "Title"`, `???`) converted to callout syntax before parsing.
//!
//! Pattern: Adapter (dialect) — a pre-pass that rewrites a foreign dialect into the syntax
//! the parser and `callout` already understand.

/// Convert mkdocs admonitions (`!!! note "Title"` + indented body) into callout syntax.
#[must_use]
pub fn convert_admonitions(src: &str) -> String {
    convert_marked(src).0
}

/// Where a raw HTML block that cannot be interrupted ends.
enum HtmlEnd {
    /// At the first line that contains this text (CommonMark HTML block types 1 to 5).
    Marker(&'static str),
    /// At the next blank line (types 6 and 7).
    Blank,
}

/// Does `line` open an HTML block? (CommonMark 4.6; type 7 is approximated by "a tag starts the
/// line" because the point is only to leave HTML content alone.)
fn html_block_start(line: &str, at_block_start: bool) -> Option<HtmlEnd> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() >= 4 || !t.starts_with('<') {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let raw = |tag: &str| {
        lower.strip_prefix(tag).is_some_and(|r| {
            r.is_empty() || r.starts_with(|c: char| c == '>' || c == ' ' || c == '\t')
        })
    };
    if raw("<pre") || raw("<script") || raw("<style") || raw("<textarea") {
        return Some(HtmlEnd::Marker(match () {
            () if raw("<pre") => "</pre>",
            () if raw("<script") => "</script>",
            () if raw("<style") => "</style>",
            () => "</textarea>",
        }));
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

/// Like [`convert_admonitions`], also returning the (0-based) lines that were admonition headers.
/// The text keeps its line count: every line stays one line.
#[must_use]
pub(super) fn convert_marked(src: &str) -> (String, Vec<usize>) {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut headers = Vec::new();
    let mut i = 0;
    let mut fence: Option<(char, usize)> = None;
    let mut html: Option<HtmlEnd> = None;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ind = line.len() - t.len();
        // raw HTML is content, not syntax
        if fence.is_none() {
            let blank_before = i == 0 || lines[i - 1].trim().is_empty();
            if html.is_none() {
                html = html_block_start(line, blank_before);
            }
            match &html {
                Some(HtmlEnd::Marker(m)) if line.contains(m) => html = None,
                Some(HtmlEnd::Blank) if line.trim().is_empty() => html = None,
                Some(_) => {
                    out.push(line.to_string());
                    i += 1;
                    continue;
                }
                None => {}
            }
        }
        // fenced code tracking (only the plain top-level / indented fences)
        if let Some(c) = t.chars().next().filter(|c| *c == '`' || *c == '~') {
            let n = t.chars().take_while(|x| *x == c).count();
            if n >= 3 {
                match fence {
                    None => fence = Some((c, n)),
                    Some((fc, fnn)) if fc == c && n >= fnn && t[n..].trim().is_empty() => {
                        fence = None;
                    }
                    _ => {}
                }
            }
        }
        if fence.is_none() && ind < 4 {
            if let Some((fold, ty, title)) = parse_admonition_header(t) {
                let base = &line[..ind];
                // body: indented >= ind+4 or blank (blank only if followed by more body)
                let mut j = i + 1;
                let mut body: Vec<&str> = Vec::new();
                while j < lines.len() {
                    let l = lines[j];
                    if l.trim().is_empty() {
                        let mut k = j;
                        while k < lines.len() && lines[k].trim().is_empty() {
                            k += 1;
                        }
                        if k < lines.len() && indent_of(lines[k]) >= ind + 4 {
                            body.extend(&lines[j..k]);
                            j = k;
                            continue;
                        }
                        break;
                    }
                    if indent_of(l) >= ind + 4 {
                        body.push(l);
                        j += 1;
                    } else {
                        break;
                    }
                }
                // a header without a body is a plain line, not an admonition
                if body.iter().any(|l| !l.trim().is_empty()) {
                    let mut hdr = format!(
                        "{}> [!{}]{}",
                        base,
                        ty,
                        fold.map(|c| c.to_string()).unwrap_or_default()
                    );
                    if !title.is_empty() {
                        hdr.push(' ');
                        hdr.push_str(&title);
                    }
                    headers.push(out.len());
                    out.push(hdr);
                    for l in body {
                        if l.trim().is_empty() {
                            out.push(format!("{base}>"));
                        } else {
                            out.push(format!("{}> {}", base, &l[ind + 4..]));
                        }
                    }
                    i = j;
                    continue;
                }
            }
        }
        out.push(line.to_string());
        i += 1;
    }
    (out.join("\n"), headers)
}

fn indent_of(l: &str) -> usize {
    l.len() - l.trim_start_matches(' ').len()
}

fn parse_admonition_header(t: &str) -> Option<(Option<char>, String, String)> {
    let (rest, fold) = if let Some(r) = t.strip_prefix("!!!") {
        (r, None)
    } else if let Some(r) = t.strip_prefix("???+") {
        (r, Some('+'))
    } else {
        (t.strip_prefix("???")?, Some('-'))
    };
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim();
    let ty: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if ty.is_empty() {
        return None;
    }
    let after = rest[ty.len()..].trim();
    let title = if after.is_empty() {
        String::new()
    } else if after.starts_with('"') && after.ends_with('"') && after.len() >= 2 {
        after[1..after.len() - 1].to_string()
    } else {
        return None;
    };
    Some((fold, ty.to_lowercase(), title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_admonitions_and_skips_fences() {
        assert_eq!(
            convert_admonitions("!!! warning \"Careful\"\n    Mind.\n\nafter"),
            "> [!warning] Careful\n> Mind.\n\nafter"
        );
        assert_eq!(convert_admonitions("??? tip\n    x"), "> [!tip]-\n> x");
        let fenced = "```\n!!! note\n    x\n```";
        assert_eq!(convert_admonitions(fenced), fenced);
    }
}
