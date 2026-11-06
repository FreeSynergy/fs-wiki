//! Placeholder scanning.
//!
//! Pattern: Iterator-style scanner — finds `{{…}}` ranges (char offsets) without evaluating.

/// A `{{…}}` placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placeholder {
    /// Start of the char range including the braces.
    pub start: usize,
    /// End of the char range.
    pub end: usize,
    /// Variable name (`date` for dates).
    pub name: String,
    /// `Some(fmt)` for `{{date:fmt}}`.
    pub date_format: Option<String>,
}

/// Find placeholders in `text` (char offsets). `\{{` is not a placeholder.
#[must_use]
pub fn find_placeholders(text: &str) -> Vec<Placeholder> {
    let chars: Vec<char> = text.chars().collect();
    let mut v = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '{' && !(i > 0 && chars[i - 1] == '\\') {
            if let Some(rel) = (i + 2..chars.len().saturating_sub(1))
                .find(|&j| chars[j] == '}' && chars[j + 1] == '}')
            {
                let inner: String = chars[i + 2..rel].iter().collect();
                let inner_t = inner.trim();
                let ok = !inner_t.is_empty() && !inner.contains('\n') && !inner.contains('{');
                if ok {
                    let (name, fmt) = match inner_t.split_once(':') {
                        Some(("date", f)) => ("date".to_string(), Some(f.to_string())),
                        _ => (inner_t.to_string(), None),
                    };
                    v.push(Placeholder {
                        start: i,
                        end: rel + 2,
                        name,
                        date_format: fmt,
                    });
                    i = rel + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    v
}

/// Names of all variables a template needs (`date` placeholders excluded).
#[must_use]
pub fn variables(text: &str) -> Vec<String> {
    let mut names: Vec<String> = find_placeholders(text)
        .into_iter()
        .filter(|p| p.date_format.is_none())
        .map(|p| p.name)
        .collect();
    names.sort();
    names.dedup();
    names
}
