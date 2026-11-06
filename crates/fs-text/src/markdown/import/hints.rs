//! Source-style hints recorded so the exporter can reproduce a file.
//!
//! Pattern: Memento (of style) — table dash widths and `_` delimiters are noted at
//! import and read back at export.

use crate::mark::MarkValue;

/// Widths of the delimiter row segments (`|---|:--:|`), to write tables back as they were.
pub(super) fn table_dashes(table_src: &str) -> Option<String> {
    let line = table_src.lines().nth(1)?;
    let t = line.trim().trim_start_matches('>').trim();
    // the outer pipes are optional in GFM; the canonical writer always writes them
    let spaced = t.starts_with('|') && t[1..].starts_with(' ');
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    Some(format!(
        "{}{}",
        if spaced { "s" } else { "" },
        t.split('|')
            .map(|c| c.trim().chars().count().to_string())
            .collect::<Vec<_>>()
            .join(",")
    ))
}

/// Remember underscore delimiters (`_em_`, `__strong__`) so export can write them back.
pub(super) fn under(src: &str, at: usize) -> MarkValue {
    if src[at..].starts_with('_') {
        MarkValue::Name("_".into())
    } else {
        MarkValue::None
    }
}
