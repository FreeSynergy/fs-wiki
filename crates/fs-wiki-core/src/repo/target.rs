//! Where a link points: the path it means, whatever way it is written.
//!
//! Pattern: Value object + pure functions — `[[wiki/links]]` are literal paths; a Markdown link
//! `[x](../a/b.md)` is relative to the page it stands on, as in every Markdown reader. This file
//! turns a written link into the path it means (`Written`) and back into text for another place.

use fs_text::{LinkStyle, LinkValue};

/// How a link was written and what it means.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Written {
    /// The path of the page or folder meant, no fragment, no trailing `/`.
    pub target: String,
    form: Form,
    /// The written form had a trailing `/` (a folder).
    slash: bool,
    /// The written form began with `./`.
    dot: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Form {
    /// `[[path]]`: the path as is.
    Literal,
    /// `/path`: from the root.
    Absolute,
    /// `a/b.md`, `./b.md`, `../b.md`: from the folder of the page it stands on.
    Relative,
}

/// The folder of `page` (everything before the last `/`), as segments.
fn folder_of(page: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = page.split('/').collect();
    parts.pop();
    parts
}

/// `written` (a page part, no fragment) resolved against the folder `from`; `None` if it leaves
/// the root.
fn join(from: Vec<&str>, written: &str) -> Option<String> {
    let mut parts = from;
    for part in written.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// What `link`, written on the page `source`, means; `None` for external links and anchors.
pub(super) fn written(link: &LinkValue, source: &str) -> Option<Written> {
    if !link.is_internal() {
        return None;
    }
    let page = link.split_fragment().0;
    if page.is_empty() {
        return None;
    }
    let (form, target) = match link.style {
        LinkStyle::Wiki | LinkStyle::Embed => (Form::Literal, page.to_string()),
        _ if page.starts_with('/') => (Form::Absolute, join(vec![], page)?),
        _ => (Form::Relative, join(folder_of(source), page)?),
    };
    (!target.is_empty()).then(|| Written {
        target,
        form,
        slash: page.ends_with('/'),
        dot: page.starts_with("./"),
    })
}

impl Written {
    /// The paths this link can lead to, in the order a reader tries them: the path, the path with
    /// `.md`, and the `index.md` / `README.md` of a folder of that name.
    pub(super) fn candidates(&self) -> Vec<String> {
        let t = &self.target;
        let mut all = vec![t.clone()];
        if !t.ends_with(".md") {
            all.push(format!("{t}.md"));
        }
        all.push(format!("{t}/index.md"));
        all.push(format!("{t}/README.md"));
        all
    }

    /// Whether the written form had a trailing `/`.
    pub(super) fn slash(&self) -> bool {
        self.slash
    }

    /// The same link, meaning `target`, written on the page `source`; `slash` keeps a trailing `/`.
    pub(super) fn render(&self, target: &str, source: &str, slash: bool) -> String {
        let mut text = match self.form {
            Form::Literal => target.to_string(),
            Form::Absolute => format!("/{target}"),
            Form::Relative => relative(&folder_of(source), target, self.dot),
        };
        if slash && self.form != Form::Literal && !text.ends_with('/') {
            text.push('/');
        }
        text
    }
}

/// `target` seen from the folder `from`.
fn relative(from: &[&str], target: &str, dot: bool) -> String {
    let to: Vec<&str> = target.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from.len() - common;
    let mut parts: Vec<&str> = vec![".."; ups];
    parts.extend(&to[common..]);
    if parts.is_empty() {
        return ".".into();
    }
    let text = parts.join("/");
    if ups == 0 && dot {
        format!("./{text}")
    } else {
        text
    }
}
