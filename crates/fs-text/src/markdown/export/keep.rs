//! What a changed block keeps from the file it was read from.
//!
//! Pattern: Helpers of the Memento — the line ending, the blank lines around a block and the
//! check for HTML that a glued line would join.

use super::options::ExportOptions;
use crate::doc::{Block, BlockKind};

/// Options for one block: a callout that was read as an admonition stays one.
pub(super) fn opts_for(b: &Block, opts: ExportOptions) -> ExportOptions {
    ExportOptions {
        admonitions: opts.admonitions || b.attrs.extra.contains_key("admonition"),
        ..opts
    }
}

/// The line ending of the file the blocks were read from (`\r\n` if a kept source has one).
pub(super) fn line_ending(blocks: &[Block]) -> &'static str {
    let crlf = blocks
        .iter()
        .filter_map(|b| b.attrs.source.as_ref())
        .any(|s| s.text.contains("\r\n"));
    if crlf {
        "\r\n"
    } else {
        "\n"
    }
}

/// An HTML block that goes on until a blank line (everything but a comment, a processing
/// instruction, CDATA or a closed `pre` / `script` / `style` / `textarea`): a line glued under
/// it would belong to it.
pub(super) fn open_html(b: &Block) -> bool {
    // a kept link definition is stored as an HTML-kind block, but it ends with its line
    if b.attrs.kind != BlockKind::Html || b.attrs.extra.contains_key("unparsed") {
        return false;
    }
    let t = b.text.to_string().to_ascii_lowercase();
    let t = t.trim();
    let closed = [
        ("<!--", "-->"),
        ("<?", "?>"),
        ("<![cdata[", "]]>"),
        ("<pre", "</pre>"),
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<textarea", "</textarea>"),
    ];
    !closed
        .iter()
        .any(|(open, close)| t.starts_with(open) && t.ends_with(close))
}

/// A line of spaces and tabs only (a no-break space is text).
fn blank(l: &str) -> bool {
    l.chars().all(|c| matches!(c, ' ' | '\t' | '\r'))
}

/// What stood in front of a changed block in the file (blank lines, a second definition of a
/// label, …), if the block still follows the block it followed there. The comment that keeps two
/// lists apart is written by the exporter itself and is not repeated.
pub(super) fn kept_lead(b: &Block, prev: Option<&Block>) -> Option<Vec<String>> {
    let s = b.attrs.source.as_ref()?;
    if s.prev != prev.map(|p| p.id) || s.own == 0 {
        return None;
    }
    let lead: Vec<String> = s.text[..s.own]
        .lines()
        .filter(|l| l.trim() != "<!-- -->")
        .map(String::from)
        .collect();
    (!lead.is_empty()).then_some(lead)
}

/// The blank lines that ended the file after a changed last block.
pub(super) fn kept_tail(b: &Block) -> Vec<String> {
    let Some(s) = b.attrs.source.as_ref() else {
        return Vec::new();
    };
    let mut v: Vec<String> = s
        .text
        .lines()
        .rev()
        .take_while(|l| blank(l))
        .map(String::from)
        .collect();
    v.reverse();
    if v.len() == s.text.lines().count() {
        v.clear();
    }
    v
}

/// Write the canonical lines gathered so far to `res`.
pub(super) fn flush(res: &mut String, lines: &mut Vec<String>, eol: &str) {
    if lines.is_empty() {
        return;
    }
    if !res.is_empty() && !res.ends_with(['\n', '\r']) {
        res.push_str(eol);
    }
    // the kept source before already ends with a blank line: do not add a second one
    let ends_blank = res.ends_with("\n\n") || res.ends_with("\r\n\r\n");
    let skip = usize::from(ends_blank && lines[0].is_empty());
    for l in &lines[skip..] {
        res.push_str(l);
        res.push_str(eol);
    }
    lines.clear();
}
