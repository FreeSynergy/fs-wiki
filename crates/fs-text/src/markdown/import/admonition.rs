//! mkdocs admonitions (`!!! note "Title"`, `???`) converted to callout syntax before parsing.
//!
//! Pattern: Adapter (dialect) — a pre-pass that rewrites a foreign dialect into the syntax
//! the parser and `callout` already understand.

/// Convert mkdocs admonitions (`!!! note "Title"` + indented body) into callout syntax.
#[must_use]
pub fn convert_admonitions(src: &str) -> String {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    let mut fence: Option<(char, usize)> = None;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ind = line.len() - t.len();
        // fenced code tracking (only the plain top-level / indented fences)
        if let Some(c) = t.chars().next().filter(|c| *c == '`' || *c == '~') {
            let n = t.chars().take_while(|x| *x == c).count();
            if n >= 3 {
                match fence {
                    None => fence = Some((c, n)),
                    Some((fc, fnn)) if fc == c && n >= fnn && t[n..].trim().is_empty() => {
                        fence = None
                    }
                    _ => {}
                }
            }
        }
        if fence.is_none() && ind < 4 {
            if let Some((fold, ty, title)) = parse_admonition_header(t) {
                let mut hdr = format!(
                    "{}> [!{}]{}",
                    &line[..ind],
                    ty,
                    fold.map(|c| c.to_string()).unwrap_or_default()
                );
                if !title.is_empty() {
                    hdr.push(' ');
                    hdr.push_str(&title);
                }
                out.push(hdr);
                i += 1;
                let base = &line[..ind];
                // body: indented >= ind+4 or blank (blank only if followed by more body)
                let mut body: Vec<&str> = Vec::new();
                while i < lines.len() {
                    let l = lines[i];
                    if l.trim().is_empty() {
                        // lookahead
                        let mut j = i;
                        while j < lines.len() && lines[j].trim().is_empty() {
                            j += 1;
                        }
                        if j < lines.len() && indent_of(lines[j]) >= ind + 4 {
                            body.extend(&lines[i..j]);
                            i = j;
                            continue;
                        }
                        break;
                    }
                    if indent_of(l) >= ind + 4 {
                        body.push(l);
                        i += 1;
                    } else {
                        break;
                    }
                }
                for l in body {
                    if l.trim().is_empty() {
                        out.push(format!("{}>", base));
                    } else {
                        out.push(format!("{}> {}", base, &l[ind + 4..]));
                    }
                }
                continue;
            }
        }
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
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
