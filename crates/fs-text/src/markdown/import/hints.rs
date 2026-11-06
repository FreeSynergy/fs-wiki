//! Source-style hints recorded so the exporter can reproduce a file.
//!
//! Pattern: Memento (of style) — glue, table dash widths and `_` delimiters are noted at
//! import and read back at export.

use crate::mark::MarkValue;

/// Was the line before `off` non-blank (block glued to its predecessor)?
pub(super) fn glue_before(src: &str, off: usize) -> bool {
    let ls = src[..off].rfind('\n').map_or(0, |i| i + 1);
    if ls == 0 {
        return false;
    }
    let prev = src[..ls - 1].rsplit('\n').next().unwrap_or("");
    !prev.trim_start_matches([' ', '>', '\t']).trim().is_empty()
}

/// Widths of the delimiter row segments (`|---|:--:|`), to write tables back as they were.
pub(super) fn table_dashes(table_src: &str) -> Option<String> {
    let line = table_src.lines().nth(1)?;
    let t = line.trim().trim_start_matches('>').trim();
    let t = t.strip_prefix('|')?.strip_suffix('|')?;
    let spaced = if t.starts_with(' ') { "s" } else { "" };
    Some(format!(
        "{spaced}{}",
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
