//! What `mkdocs.yml` says about the order: the numbered `nav_translations` and the files of a `nav:` list.
//!
//! Pattern: Adapter (YAML value to plain data) — small pure readers over a parsed `mkdocs.yml`.

use std::collections::HashMap;

use yaml_serde::Value;

use crate::nav::is_md;

/// The numbers the configuration gives to names: `nav_translations: {"6 Manifesto": …}` says that `manifesto` is sixth.
pub(crate) fn numbered_titles(config: &Value) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    let mut stack = vec![config];
    while let Some(v) = stack.pop() {
        match v {
            Value::Mapping(m) => {
                for (k, x) in m {
                    if k.as_str() == Some("nav_translations")
                        && let Some(t) = x.as_mapping()
                    {
                        for key in t.keys().filter_map(Value::as_str) {
                            if let Some((n, name)) = split_number(key) {
                                out.entry(name.to_lowercase().replace(' ', "-"))
                                    .or_insert(n);
                            }
                        }
                    }
                    stack.push(x);
                }
            }
            Value::Sequence(s) => stack.extend(s),
            _ => {}
        }
    }
    out
}

/// `"3 Components"` is (3, `Components`).
pub(crate) fn split_number(title: &str) -> Option<(u32, &str)> {
    let (n, rest) = title.split_once(' ')?;
    Some((n.parse().ok()?, rest.trim()))
}

/// The files a `nav:` list names, in its order (titles and nesting do not matter here).
pub(crate) fn listed_files(nav: &Value) -> Vec<String> {
    let mut out = Vec::new();
    match nav {
        Value::Sequence(items) => {
            for item in items {
                out.extend(listed_files(item));
            }
        }
        Value::Mapping(m) => {
            for (_, v) in m {
                out.extend(listed_files(v));
            }
        }
        Value::String(s) if is_md(s) => out.push(s.trim_start_matches("./").to_string()),
        _ => {}
    }
    out
}
