//! Following a link of a Markdown page.
//!
//! Pattern: Strategy (pure function with the lookup passed in) — `fs-wiki-core` matches a link
//! only when its path equals a text's path literally; documentation links are relative and end in
//! `.md` (`../data/index.md#top`), so the app resolves them here, against the page they are on.
//! The decision is in the app (not in `fs-wiki-core`) for now; see DESIGN.md.

use fs_components_workbenches::LinkTarget;

/// Where `href`, written on the page `from`, leads. `exists` tells whether a page is there.
pub fn resolve(from: &str, href: &str, exists: &dyn Fn(&str) -> bool) -> LinkTarget {
    if is_external(href) {
        return LinkTarget::External(href.to_string());
    }
    let page = href.split('#').next().unwrap_or("");
    let page = page.split('?').next().unwrap_or("");
    if page.is_empty() {
        return LinkTarget::Page(from.to_string());
    }
    let decoded = percent_decode(page);
    let Some(path) = join(from, &decoded) else {
        return LinkTarget::Missing(href.to_string());
    };
    let dir = path.trim_end_matches('/');
    let candidates = [
        path.clone(),
        format!("{dir}.md"),
        format!("{dir}/index.md"),
        format!("{dir}/README.md"),
    ];
    candidates
        .into_iter()
        .find(|c| !c.is_empty() && !c.ends_with('/') && exists(c))
        .map_or_else(|| LinkTarget::Missing(path), LinkTarget::Page)
}

fn is_external(href: &str) -> bool {
    href.contains("://") || href.starts_with("mailto:") || href.starts_with("tel:")
}

/// `target` relative to the folder of `from`, with `.` and `..` resolved; `None` if it leaves the
/// root. A leading `/` counts from the root.
fn join(from: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        let mut dir: Vec<&str> = from.split('/').collect();
        dir.pop();
        dir
    };
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    let mut out = parts.join("/");
    if target.ends_with('/') && !out.is_empty() {
        out.push('/');
    }
    Some(out)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
