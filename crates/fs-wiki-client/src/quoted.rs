//! Reading facts back out of the text of a problem.
//!
//! Pattern: none, two free functions. The server's error texts show paths with `{:?}` (a Rust string
//! literal); these read them back, escapes included.

/// The string literals in `text`, in order, unescaped (`"a\"b"` gives `a"b`).
pub(crate) fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut one = String::new();
        let mut closed = false;
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    closed = true;
                    break;
                }
                '\\' => one.push(escaped(&mut chars)),
                other => one.push(other),
            }
        }
        if closed {
            out.push(one);
        }
    }
    out
}

/// The character behind a backslash: the escapes `{:?}` writes.
fn escaped(chars: &mut std::str::Chars<'_>) -> char {
    match chars.next() {
        Some('n') => '\n',
        Some('t') => '\t',
        Some('r') => '\r',
        Some('0') => '\0',
        Some('u') => {
            let hex: String = chars
                .skip_while(|c| *c == '{')
                .take_while(|c| *c != '}')
                .collect();
            u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER)
        }
        Some(other) => other,
        None => '\\',
    }
}

/// The run of digits right behind `marker` in `text`.
pub(crate) fn text_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let rest = &text[text.find(marker)? + marker.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    Some(&rest[..end]).filter(|n| !n.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_come_back_with_their_escapes() {
        let text = format!("cannot move {:?} to {:?}: x", "a\"b\\c", "ü\n\u{1}");
        assert_eq!(quoted(&text), ["a\"b\\c", "ü\n\u{1}"]);
        assert!(quoted("no quotes").is_empty());
        assert!(quoted("open \"never closed").is_empty());
    }

    #[test]
    fn digits_after_a_marker() {
        assert_eq!(text_after("has revision 42, not 7", " has revision "), None);
        assert_eq!(
            text_after("x has revision 42, not 7", " has revision "),
            Some("42")
        );
        assert_eq!(text_after("x has revision ", " has revision "), None);
    }
}
