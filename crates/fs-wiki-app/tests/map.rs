//! The mapping from `fs-text` documents to the neutral blocks.
//!
//! Pattern: Example-based tests over real Markdown.

use fs_components_widgets::{Block, MarkKind};
use fs_text::import_markdown;
use fs_wiki_app::to_document;

fn blocks(md: &str) -> Vec<Block> {
    to_document("t", &import_markdown(md)).blocks
}

#[test]
fn headings_paragraphs_and_marks_keep_their_offsets() {
    let b = blocks("# Title\n\nSome **bold** and `code` and [link](a.md) text.\n");
    assert!(matches!(&b[0], Block::Heading { level: 1, .. }));
    let Block::Paragraph(p) = &b[1] else {
        panic!("{b:?}")
    };
    assert_eq!(p.text, "Some bold and code and link text.");
    let strong = p.marks.iter().find(|m| m.kind == MarkKind::Strong).unwrap();
    assert_eq!(&p.text[strong.start..strong.end], "bold");
    let link = p
        .marks
        .iter()
        .find(|m| matches!(m.kind, MarkKind::Link { .. }))
        .unwrap();
    assert_eq!(&p.text[link.start..link.end], "link");
    assert_eq!(
        link.kind,
        MarkKind::Link {
            href: "a.md".into()
        }
    );
}

#[test]
fn nested_lists_quotes_and_tasks() {
    let b = blocks("1. one\n2. two\n   - inner\n\n> quoted\n\n- [x] done\n");
    let Block::List { ordered, items, .. } = &b[0] else {
        panic!("{b:?}")
    };
    assert!(*ordered);
    assert_eq!(items.len(), 2);
    assert!(
        items[1]
            .blocks
            .iter()
            .any(|x| matches!(x, Block::List { ordered: false, .. }))
    );
    assert!(matches!(&b[1], Block::Quote(_)));
    let Block::List { items, .. } = &b[2] else {
        panic!("{b:?}")
    };
    assert_eq!(items[0].task, Some(true));
}

#[test]
fn tables_carry_the_marks_of_their_cells() {
    let b = blocks("| A | B |\n|---|---|\n| x | **y** |\n");
    let Block::Table { rows, .. } = &b[0] else {
        panic!("{b:?}")
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][0].text, "x");
    assert_eq!(rows[1][1].text, "y");
    assert_eq!(rows[1][1].marks.len(), 1);
    assert_eq!(rows[1][1].marks[0].kind, MarkKind::Strong);
}

#[test]
fn code_front_matter_and_html_are_shown_not_dropped() {
    let b = blocks("---\ntitle: T\n---\n\n```rust\nfn x() {}\n```\n\n<div>raw html</div>\n");
    assert!(matches!(&b[0], Block::Code { lang, .. } if lang == "yaml"));
    assert!(matches!(&b[1], Block::Code { lang, text } if lang == "rust" && text.contains("fn x")));
    assert!(b.iter().any(|x| x.plain_text().contains("raw html")));
}

#[test]
fn footnote_references_are_written_out() {
    let b = blocks("Text[^1] here.\n\n[^1]: the note\n");
    assert!(b[0].plain_text().contains("[^1]"), "{b:?}");
    assert!(b.iter().any(|x| x.plain_text().contains("the note")));
}

#[test]
fn deeply_nested_quotes_and_lists_do_not_overflow() {
    let deep = ">".repeat(1000) + " bottom\n";
    let b = blocks(&deep);
    assert!(b.iter().any(|x| x.plain_text().contains("bottom")));
    let nested = (0..300)
        .map(|i| format!("{}- item\n", "  ".repeat(i)))
        .collect::<String>();
    let b = blocks(&nested);
    assert!(!b.is_empty());
}

#[test]
fn a_soft_break_is_a_space_and_a_hard_break_is_a_line_break() {
    let soft = blocks("first line\nsecond **line** here\n");
    let Block::Paragraph(p) = &soft[0] else {
        panic!("{soft:?}")
    };
    assert_eq!(p.text, "first line second line here");
    let strong = p.marks.iter().find(|m| m.kind == MarkKind::Strong).unwrap();
    assert_eq!(
        &p.text[strong.start..strong.end],
        "line",
        "marks stay on their text"
    );
    for hard in ["one  \ntwo\n", "one\\\ntwo\n"] {
        let b = blocks(hard);
        let Block::Paragraph(p) = &b[0] else {
            panic!("{b:?}")
        };
        assert_eq!(p.text, "one\ntwo", "{hard:?}");
    }
}
