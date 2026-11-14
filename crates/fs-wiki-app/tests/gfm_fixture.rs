//! The text of the user of 2026-11-14 as the reader's blocks: table alignment, task items, a checkbox in a table cell (our
//! extension; GFM has task items in lists only), strike-through (`~x~` and `~~x~~`) and emphasis (`_x_` and `*x*`).
//!
//! Pattern: Example-based test over the fixture of fs-text (`fs-text/tests/corpus/user-gfm-2026-11-14.md`).

use fs_components_widgets::{Align, Block, MarkKind};
use fs_text::import_markdown;
use fs_wiki_app::to_document;

const FIXTURE: &str = include_str!("../../fs-text/tests/corpus/user-gfm-2026-11-14.md");

#[test]
fn the_reader_gets_alignment_tasks_marks_and_the_checkbox_cells() {
    let blocks = to_document("t", &import_markdown(FIXTURE)).blocks;
    let Block::Table { align, rows } = blocks
        .iter()
        .find(|b| matches!(b, Block::Table { .. }))
        .expect("a table")
    else {
        unreachable!()
    };
    assert_eq!(align, &[Align::Center, Align::Left, Align::Right]);
    // The checkbox stays the text `[ ]` / `[x]` in its cell: the reader draws it as a box, saving writes it back as is.
    assert_eq!(rows[1][2].text, "[ ]");
    assert_eq!(rows[2][2].text, "[x]");
    let Block::List { items, .. } = blocks
        .iter()
        .find(|b| matches!(b, Block::List { .. }))
        .expect("a list")
    else {
        unreachable!()
    };
    let tasks: Vec<Option<bool>> = items.iter().map(|i| i.task).collect();
    assert_eq!(tasks, [Some(false), Some(true), None]);
    let Block::Paragraph(p) = blocks
        .iter()
        .find(|b| matches!(b, Block::Paragraph(_)))
        .expect("a paragraph")
    else {
        unreachable!()
    };
    let count = |kind: MarkKind| p.marks.iter().filter(|m| m.kind == kind).count();
    assert_eq!(count(MarkKind::Strike), 2, "~x~ and ~~x~~");
    assert_eq!(count(MarkKind::Emphasis), 2, "_x_ and *x*");
}
