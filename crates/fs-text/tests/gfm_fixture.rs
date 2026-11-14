//! The text of the user of 2026-11-14 (headings, a paragraph with strike-through and emphasis, a table with alignment and
//! checkboxes in a cell, a task list), as a fixture: it goes through the model and back byte for byte, also through the
//! canonical writer alone (no kept source), and the model knows what it is (alignment, task state, marks).
//!
//! Pattern: Example-based tests over a real text. The same text is the user fixture of the conformance collection of the
//! editor building blocks in fs-components (`fs-components-editor`).

mod common;
use fs_text::*;

const FIXTURE: &str = include_str!("corpus/user-gfm-2026-11-14.md");

#[test]
fn the_text_comes_back_byte_for_byte_when_it_is_not_changed() {
    let doc = import_markdown(FIXTURE);
    assert_eq!(export_markdown(&doc, ExportOptions::default()), FIXTURE);
}

/// A block that is not touched keeps its text as written, also when another block is edited and the text is saved.
#[test]
fn an_edit_elsewhere_leaves_every_other_block_as_written() {
    let mut doc = import_markdown(FIXTURE);
    let heading = doc
        .blocks
        .iter()
        .position(|b| b.text.to_string() == "Heading two")
        .expect("the second heading");
    doc.blocks[heading].insert(0, "X ");
    let out = export_markdown(&doc, ExportOptions::default());
    assert_eq!(out, FIXTURE.replace("## Heading two", "## X Heading two"));
    assert!(
        out.contains("~struck~, ~~struck too~~"),
        "the paragraph keeps both strike spellings"
    );
}

/// Stated limit: the writer alone (a block without its kept source, e.g. a block that was edited itself) writes
/// strike-through in the GFM form `~~x~~`; `~x~` is read as strike-through and written as `~~x~~`. Everything else of
/// the fixture comes out the same.
#[test]
fn the_writer_alone_normalises_only_the_single_tilde() {
    let doc = import_markdown(FIXTURE);
    assert_eq!(
        common::canonical(&doc),
        FIXTURE.replace("~struck~,", "~~struck~~,")
    );
}

#[test]
fn the_model_knows_alignment_tasks_and_marks() {
    let doc = import_markdown(FIXTURE);
    let table = doc
        .blocks
        .iter()
        .find_map(|b| match &b.attrs.kind {
            BlockKind::Table { align } => Some(align.clone()),
            _ => None,
        })
        .expect("a table");
    assert_eq!(table, [Align::Center, Align::Left, Align::Right]);
    let tasks: Vec<Option<bool>> = doc
        .blocks
        .iter()
        .filter_map(|b| {
            b.attrs.containers.iter().find_map(|c| match c {
                Container::Item { task, .. } => Some(*task),
                _ => None,
            })
        })
        .collect();
    assert_eq!(tasks, [Some(false), Some(true), None]);
}
