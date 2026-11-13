//! The title of a page, by one rule.
//!
//! Pattern: Strategy (fallback order) — the first usable of: the `title:` of the front matter (it wins), the plain text of
//! the first level-1 heading, the name the wiki derived (`fs-wiki-core`, `repo/name.rs`). Plain text means the words without
//! their marks (`# Foo *bar*` is "Foo bar"), a line break inside the heading a space. The layouts of `fs-components` draw the
//! title big and leave the first heading out when it says the same (`leads_with_title`), so the title appears once: this
//! rule makes the title and that heading come from the same text.

use fs_text::{Block, BlockKind, Document as Doc};

use super::inline::inline_of;

/// The longest title taken from the front matter (characters), as in the wiki's derived name.
const MAX_CHARS: usize = 80;

/// The title of the page of `doc`; `name` is the wiki's derived name, the last resort.
pub(super) fn page_title(name: &str, doc: &Doc) -> String {
    front_title(doc)
        .or_else(|| heading_title(doc))
        .unwrap_or_else(|| name.trim().to_owned())
}

fn front_title(doc: &Doc) -> Option<String> {
    let block = doc
        .blocks
        .iter()
        .find(|b| b.attrs.kind == BlockKind::FrontMatter)?;
    let text = block.text.to_string();
    let value = text.lines().find_map(|l| l.strip_prefix("title:"))?;
    let title = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    (!title.is_empty()).then(|| title.chars().take(MAX_CHARS).collect())
}

/// The plain text of the first level-1 heading that is not inside a quote or a list. It is not cut: a heading longer than the
/// wiki's name (80 characters) must still be equal to the title, or the layout would draw both.
fn heading_title(doc: &Doc) -> Option<String> {
    let block: &Block = doc.blocks.iter().find(|b| {
        b.attrs.kind == (BlockKind::Heading { level: 1 }) && b.attrs.containers.is_empty()
    })?;
    let text = inline_of(block).text;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_text::import_markdown;

    fn title(markdown: &str, name: &str) -> String {
        page_title(name, &import_markdown(markdown))
    }

    #[test]
    fn the_front_matter_wins_then_the_first_h1_then_the_name() {
        assert_eq!(title("---\ntitle: From front\n---\n# Heading\n", "n"), "From front");
        assert_eq!(title("---\ntitle: \"Quoted: yes\"\n---\n", "n"), "Quoted: yes");
        assert_eq!(title("# Handbook\n\ntext\n", "x"), "Handbook");
        assert_eq!(title("text only\n", "the name"), "the name");
        assert_eq!(title("## second level\n", "second level"), "second level");
    }

    #[test]
    fn marks_in_the_heading_are_plain_text() {
        assert_eq!(title("# Foo *bar* `baz`\n", "n"), "Foo bar baz");
        assert_eq!(title("#  Spaced out  \n", "n"), "Spaced out");
    }

    #[test]
    fn the_first_h1_counts_not_a_heading_of_another_level_before_it() {
        assert_eq!(title("## Intro\n\n# Real\n", "Intro"), "Real");
        assert_eq!(title("> # quoted\n\n# Real\n", "n"), "Real");
    }

    #[test]
    fn a_long_heading_is_not_cut() {
        let long = "w".repeat(120);
        assert_eq!(title(&format!("# {long}\n"), "x"), long);
    }
}
