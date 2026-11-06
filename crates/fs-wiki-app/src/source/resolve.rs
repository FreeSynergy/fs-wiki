//! Following a link of a Markdown page.
//!
//! Pattern: Strategy (pure function with the lookups passed in) — `fs-wiki-core` matches a link
//! only when its path equals a text's path literally; documentation links are relative and end in
//! `.md` (`../data/index.md#top`), so the app resolves them here, against the page they are on.
//!
//! Outside links are opened only for the schemes `http`, `https` and `mailto`, found by parsing
//! the start of the link; any other scheme (`file:`, `javascript:`, ...) is shown, not opened.
//! A link to a folder opens its `index.md` or `README.md`, else (provisional) the first page in
//! the folder, else it is missing. `../` at the top of the docs is the docs root.

use fs_components_workbenches::LinkTarget;

/// Schemes that may be handed to the system opener.
const OPENED: [&str; 3] = ["http", "https", "mailto"];

/// Where `href`, written on the page `from`, leads. `exists` tells whether a page is there.
pub fn resolve(from: &str, href: &str, exists: &dyn Fn(&str) -> bool) -> LinkTarget {
    resolve_with_folders(from, href, exists, &|_| None)
}

/// As [`resolve`], and a folder with no index page opens `first_in(folder)` (the folder is `""`
/// for the docs root).
pub fn resolve_with_folders(
    from: &str,
    href: &str,
    exists: &dyn Fn(&str) -> bool,
    first_in: &dyn Fn(&str) -> Option<String>,
) -> LinkTarget {
    let scheme = scheme_of(href);
    if let Some(scheme) = scheme {
        let opened = OPENED.iter().any(|s| scheme.eq_ignore_ascii_case(s));
        if opened {
            return LinkTarget::External(href.to_string());
        }
        // Not opened. `scheme://...` is shown as it is; a name like `a:b.md` may still be a page.
        if href[scheme.len() + 1..].starts_with("//") {
            return LinkTarget::Missing(href.to_string());
        }
    }
    if href.starts_with("//") {
        return LinkTarget::Missing(href.to_string());
    }
    match resolve_path(from, href, exists, first_in) {
        LinkTarget::Missing(_) if scheme.is_some() => LinkTarget::Missing(href.to_string()),
        found => found,
    }
}

fn resolve_path(
    from: &str,
    href: &str,
    exists: &dyn Fn(&str) -> bool,
    first_in: &dyn Fn(&str) -> Option<String>,
) -> LinkTarget {
    let page = href.split('#').next().unwrap_or("");
    let page = page.split('?').next().unwrap_or("");
    if page.is_empty() {
        return LinkTarget::Document(from.to_string());
    }
    let decoded = percent_decode(page);
    let Some(path) = join(from, &decoded) else {
        return LinkTarget::Missing(href.to_string());
    };
    let dir = path.trim_end_matches('/');
    let at = |name: &str| {
        if dir.is_empty() {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        }
    };
    let candidates = [
        path.clone(),
        format!("{dir}.md"),
        at("index.md"),
        at("README.md"),
    ];
    candidates
        .into_iter()
        .find(|c| !c.is_empty() && !c.ends_with('/') && c != ".md" && exists(c))
        .or_else(|| first_in(dir))
        .map_or_else(|| LinkTarget::Missing(path), LinkTarget::Document)
}

/// The scheme at the start of `href` (`https` in `https://x`), if it has one: letters, digits,
/// `+`, `-`, `.` beginning with a letter, then `:` before any `/`, `?` or `#`.
fn scheme_of(href: &str) -> Option<&str> {
    let end = href.find([':', '/', '?', '#'])?;
    if !href[end..].starts_with(':') {
        return None;
    }
    let scheme = &href[..end];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(scheme)
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
