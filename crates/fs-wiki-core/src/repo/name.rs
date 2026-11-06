//! The envelope name of a text (ADR 0032): derived from the content, never typed separately.
//!
//! Pattern: Strategy (fallback order) — the first usable of: front matter `title:`, the first
//! heading, the first line of the first text block, the last segment of the path.

use fs_text::{Block, BlockKind, Document};

const MAX_CHARS: usize = 80;

/// Name for the envelope; never empty (the store refuses an empty name).
pub(super) fn derive_name(doc: &Document, path: &str) -> String {
    front_matter_title(doc)
        .or_else(|| first_line(doc.blocks.iter().find(|b| is_heading(b))))
        .or_else(|| first_line(doc.blocks.iter().find(|b| is_prose(b))))
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path).to_string())
}

fn is_heading(b: &Block) -> bool {
    matches!(b.attrs.kind, BlockKind::Heading { .. })
}

fn is_prose(b: &Block) -> bool {
    matches!(
        b.attrs.kind,
        BlockKind::Paragraph | BlockKind::Raw | BlockKind::Code { .. }
    )
}

fn first_line(block: Option<&Block>) -> Option<String> {
    let text = block?.text.to_string();
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.chars().take(MAX_CHARS).collect())
}

fn front_matter_title(doc: &Document) -> Option<String> {
    let block = doc
        .blocks
        .iter()
        .find(|b| b.attrs.kind == BlockKind::FrontMatter)?;
    let text = block.text.to_string();
    let value = text.lines().find_map(|l| l.strip_prefix("title:"))?;
    let title = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    (!title.is_empty()).then(|| title.chars().take(MAX_CHARS).collect())
}
