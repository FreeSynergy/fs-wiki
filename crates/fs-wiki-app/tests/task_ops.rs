//! Ticking a checkbox: the operations and the bytes that come out (finding W4 of 2026-11-14). Only the box changes: a list task
//! through `SetAttrs` on the block of the item, a table cell through an `Edit` of one character; every other byte of the text
//! stays. The numbering is the one the reader's view uses (`fs_components_widgets::count_tasks`).
//!
//! Pattern: Example-based tests over the fixture of the user (`fs-text/tests/corpus/user-gfm-2026-11-14.md`) and a list with
//! a nested item.

use fs_text::{ExportOptions, Op, export_markdown, import_markdown};
use fs_wiki_app::task_ops::{NoSuchBox, tick_ops};

const FIXTURE: &str = include_str!("../../fs-text/tests/corpus/user-gfm-2026-11-14.md");

/// The text after ticking box `index` to `done`.
fn ticked(md: &str, index: usize, done: bool) -> (String, Vec<Op>) {
    let mut doc = import_markdown(md);
    let ops = tick_ops(&doc, index, done).expect("a box");
    for op in &ops {
        doc.apply(op).expect("the operation fits");
    }
    (export_markdown(&doc, ExportOptions::default()), ops)
}

#[test]
fn a_list_task_and_a_table_cell_change_only_their_box() {
    // The fixture's boxes in document order: table cells `[ ]`, `[x]`, then the list `[ ]`, `[x]`.
    let (out, ops) = ticked(FIXTURE, 0, true);
    assert_eq!(ops.len(), 1);
    assert_eq!(
        out,
        FIXTURE.replace("| one | open | [ ] |", "| one | open | [x] |")
    );
    let (out, _) = ticked(FIXTURE, 1, false);
    assert_eq!(
        out,
        FIXTURE.replace("| two | done | [x] |", "| two | done | [ ] |")
    );
    let (out, ops) = ticked(FIXTURE, 2, true);
    assert!(
        matches!(ops[0], Op::SetAttrs { .. }),
        "a list task is an attribute of its item: {ops:?}"
    );
    assert_eq!(out, FIXTURE.replace("- [ ] open task", "- [x] open task"));
    let (out, _) = ticked(FIXTURE, 3, false);
    assert_eq!(out, FIXTURE.replace("- [x] done task", "- [ ] done task"));
}

#[test]
fn a_box_that_has_the_state_makes_no_operation_and_a_missing_box_is_refused() {
    let doc = import_markdown(FIXTURE);
    assert_eq!(tick_ops(&doc, 0, false), Ok(Vec::new()));
    assert_eq!(tick_ops(&doc, 4, true), Err(NoSuchBox(4)));
}

#[test]
fn nested_items_and_items_with_several_blocks_are_counted_once() {
    let md = "- [ ] a\n  - [x] inner\n- [ ] b\n\n  second paragraph of b\n- [ ] c\n";
    let (out, _) = ticked(md, 1, false);
    assert_eq!(out, md.replace("[x] inner", "[ ] inner"));
    let (out, _) = ticked(md, 3, true);
    assert_eq!(out, md.replace("- [ ] c", "- [x] c"));
}
