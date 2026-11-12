//! Component blocks: written and read back, byte for byte, also in a page that is changed around them.
//!
//! Pattern: Example-based tests of the recogniser and of the round trip through the importer and the two writers.

mod common;

use common::canonical;
use fs_text::{
    export_markdown, find_components, import_markdown, ComponentSpec, DataSource, ExportOptions,
};
use serde_json::json;

fn spec() -> ComponentSpec {
    ComponentSpec {
        component: "fs.tasks.board".into(),
        version: 1,
        config: json!({"columns": ["todo", "doing", "done"], "note": "a -- b --> c"}),
        source: Some(DataSource {
            setting: "tasks.board.project".into(),
            params: json!({"project": "wiki"}),
        }),
    }
}

fn page() -> String {
    format!(
        "# Plan\n\nBefore.\n\n{}\nAfter, and a list:\n\n* one\n* two\n",
        spec().markdown("| Task | State |\n|---|---|\n| Write | done |\n")
    )
}

#[test]
fn a_marker_is_one_line_and_never_ends_the_comment_early() {
    let marker = spec().open_marker();
    assert_eq!(marker.lines().count(), 1);
    assert!(marker.starts_with("<!-- fs-component {") && marker.ends_with(" -->"));
    let inside = &marker["<!-- fs-component ".len()..marker.len() - " -->".len()];
    assert!(!inside.contains("--"), "{inside}");
    assert_eq!(
        ComponentSpec::from_marker(&marker).unwrap().unwrap(),
        spec()
    );
}

#[test]
fn a_page_with_a_component_block_comes_back_byte_for_byte() {
    let src = page();
    let doc = import_markdown(&src);
    assert_eq!(export_markdown(&doc, ExportOptions::default()), src);
    let (found, problems) = find_components(&doc);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].spec, spec());
    // The fallback is ordinary content: the table, which a reader without the component sees.
    let inner: Vec<String> = found[0]
        .fallback()
        .map(|i| doc.blocks[i].text.to_string())
        .collect();
    assert!(inner.iter().any(|t| t.contains("Write")), "{inner:?}");
    // The canonical writer alone (what a changed block gets) keeps the component and reads back the same.
    let again = import_markdown(&canonical(&doc));
    assert_eq!(find_components(&again).0.len(), 1);
    assert_eq!(find_components(&again).0[0].spec, spec());
    assert_eq!(canonical(&again), canonical(&doc), "stable");
}

#[test]
fn blocks_around_the_component_can_change_and_it_stays() {
    let src = page().replace("Before.", "Before, and more.");
    let (found, _) = find_components(&import_markdown(&src));
    assert_eq!(found[0].spec, spec());
}

#[test]
fn a_marker_that_does_not_read_or_is_not_closed_stays_a_comment_and_is_named() {
    for (src, why) in [
        (
            "<!-- fs-component {\"component\": 3} -->\n\nx\n\n<!-- /fs-component -->\n",
            "does not read",
        ),
        (
            "<!-- fs-component {\"component\":\"a\",\"version\":1} -->\n\nno end\n",
            "not closed",
        ),
    ] {
        let doc = import_markdown(src);
        let (found, problems) = find_components(&doc);
        assert_eq!(found.len(), 0);
        assert!(problems[0].1.contains(why), "{problems:?}");
        assert_eq!(
            export_markdown(&doc, ExportOptions::default()),
            src,
            "kept as written"
        );
    }
    // An ordinary comment is none of this.
    let (found, problems) = find_components(&import_markdown("<!-- a note -->\n\ntext\n"));
    assert!(found.is_empty() && problems.is_empty());
}
