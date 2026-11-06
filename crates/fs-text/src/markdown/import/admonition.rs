//! mkdocs admonitions (`!!! note "Title"`, `???`) converted to callout syntax before parsing.
//!
//! Pattern: Adapter (dialect) — a pre-pass that rewrites a foreign dialect into the syntax
//! the parser and `callout` already understand.

use super::rawhtml::{html_block_start, HtmlEnd};

/// Convert mkdocs admonitions (`!!! note "Title"` + indented body) into callout syntax.
#[must_use]
pub fn convert_admonitions(src: &str) -> String {
    convert_marked(src).0
}

/// The body of an admonition whose header is on line `from - 1` (indent `ind`): the lines
/// indented by 4 more, blank lines only when more body follows. Returns the body and the index
/// of the first line after it.
fn collect_body<'a>(lines: &[&'a str], from: usize, ind: usize) -> (Vec<&'a str>, usize) {
    let mut next = from;
    let mut body: Vec<&str> = Vec::new();
    while next < lines.len() {
        let line = lines[next];
        if line.trim().is_empty() {
            let mut ahead = next;
            while ahead < lines.len() && lines[ahead].trim().is_empty() {
                ahead += 1;
            }
            if ahead < lines.len() && indent_of(lines[ahead]) >= ind + 4 {
                body.extend(&lines[next..ahead]);
                next = ahead;
                continue;
            }
            break;
        }
        if indent_of(line) >= ind + 4 {
            body.push(line);
            next += 1;
        } else {
            break;
        }
    }
    (body, next)
}

/// Fenced code tracking (only the plain top-level / indented fences): the fence after `line`.
fn track_fence(fence: Option<(char, usize)>, line: &str) -> Option<(char, usize)> {
    let text = line.trim_start();
    let Some(fc) = text.chars().next().filter(|c| *c == '`' || *c == '~') else {
        return fence;
    };
    let run = text.chars().take_while(|x| *x == fc).count();
    if run < 3 {
        return fence;
    }
    match fence {
        None => Some((fc, run)),
        Some((open, width)) if open == fc && run >= width && text[run..].trim().is_empty() => None,
        _ => fence,
    }
}

/// Like [`convert_admonitions`], also returning the (0-based) lines that were admonition headers.
/// The text keeps its line count: every line stays one line.
#[must_use]
pub(super) fn convert_marked(src: &str) -> (String, Vec<usize>) {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut headers = Vec::new();
    let mut row = 0;
    let mut fence: Option<(char, usize)> = None;
    let mut html: Option<HtmlEnd> = None;
    while row < lines.len() {
        let line = lines[row];
        let text = line.trim_start();
        let ind = line.len() - text.len();
        // raw HTML is content, not syntax
        if fence.is_none() {
            let blank_before = row == 0 || lines[row - 1].trim().is_empty();
            if html.is_none() {
                html = html_block_start(line, blank_before);
            }
            match &html {
                Some(HtmlEnd::Marker(m)) if line.contains(m) => html = None,
                Some(HtmlEnd::Blank) if line.trim().is_empty() => html = None,
                Some(_) => {
                    out.push(line.to_string());
                    row += 1;
                    continue;
                }
                None => {}
            }
        }
        fence = track_fence(fence, line);
        if fence.is_none() && ind < 4 {
            if let Some((fold, ty, title)) = parse_admonition_header(text) {
                let base = &line[..ind];
                let (body, next) = collect_body(&lines, row + 1, ind);
                // a header without a body is a plain line, not an admonition
                if body.iter().any(|b| !b.trim().is_empty()) {
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
                    for b in body {
                        if b.trim().is_empty() {
                            out.push(format!("{base}>"));
                        } else {
                            out.push(format!("{}> {}", base, &b[ind + 4..]));
                        }
                    }
                    row = next;
                    continue;
                }
            }
        }
        out.push(line.to_string());
        row += 1;
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
