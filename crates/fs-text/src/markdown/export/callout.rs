//! The header line of a callout.
//!
//! Pattern: Strategy by dialect — `> [!note]+ Title` (Obsidian) or `!!! note "Title"` (mkdocs),
//! chosen by [`ExportOptions::admonitions`].

use super::options::ExportOptions;

/// Header after the container `prefix`: `prefix` + dialect-specific header.
pub(super) fn callout_header(
    mut h: String,
    ty: &str,
    title: &str,
    fold: Option<char>,
    opts: ExportOptions,
) -> String {
    if opts.admonitions {
        h.push_str(match fold {
            None => "!!! ",
            Some('+') => "???+ ",
            Some(_) => "??? ",
        });
        h.push_str(ty);
        if !title.is_empty() {
            h.push_str(&format!(" \"{title}\""));
        }
    } else {
        h.push_str("> [!");
        h.push_str(ty);
        h.push(']');
        if let Some(f) = fold {
            h.push(f);
        }
        if !title.is_empty() {
            h.push(' ');
            h.push_str(title);
        }
    }
    h.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_dialects() {
        let o = ExportOptions::default();
        assert_eq!(
            callout_header(String::new(), "note", "Hi", Some('+'), o),
            "> [!note]+ Hi"
        );
        let a = ExportOptions {
            admonitions: true,
            ..o
        };
        assert_eq!(
            callout_header(String::new(), "note", "Hi", Some('+'), a),
            "???+ note \"Hi\""
        );
    }
}
