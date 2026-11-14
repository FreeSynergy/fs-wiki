//! The text of a page goes through the wiki and back byte for byte, also with the shapes a block editor cuts at blank lines:
//! a paragraph with a line break, a list, a code fence with a blank line inside, a table, several blocks.
//!
//! Pattern: Round-trip test over the real wiki (server, store). It covers the wiki's side of an edit in the Notion block
//! editor: the editor puts its blocks together into one Markdown text and hands it to `save`; this checks that `save` then
//! `source_text` returns that text unchanged. The putting together itself is tested in fs-components
//! (`blocks_tests.rs`: `title_and_blocks_go_through_markdown_and_back_unchanged`, `a_fence_is_one_block`).

mod common;

use fs_components_workbenches::DocumentSource;
use fs_wiki_app::WikiSource;

const SHAPES: &[&str] = &[
    "# Page\n\nA paragraph\nwith a line break.\n\n- one\n- two\n  - nested\n\nEnd.\n",
    "# Page\n\n```rust\nfn x() {}\n\nfn y() {}\n```\n\nAfter the fence.\n",
    "# Page\n\n## Section\n\n1. first\n2. second\n\n> a quote\n> over two lines\n\n### Deeper\n\nText with **bold** and `code`.\n",
    "# Page\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nAfter the table.\n",
];

#[test]
fn a_page_with_several_kinds_of_blocks_comes_back_unchanged_after_a_save() {
    let (rt, fixture) = common::wiki();
    let source = WikiSource::new(fixture.wiki(), rt.handle().clone(), vec![]);
    for (n, shape) in SHAPES.iter().enumerate() {
        let path = format!("shape-{n}.md");
        // Start from another text so that the save really changes something.
        rt.block_on(fs_wiki_core::Texts::create(
            &*fixture.wiki(),
            &path,
            "# Page\n\nold\n",
        ))
        .unwrap();
        let revision = source.source_text(&path).unwrap().revision;
        source.save(&path, shape, revision.as_deref()).unwrap();
        let back = source.source_text(&path).unwrap().text;
        assert_eq!(&back, shape, "shape {n}");
        // And again with the text it gave back: nothing changes.
        let revision = source.source_text(&path).unwrap().revision;
        source.save(&path, &back, revision.as_deref()).unwrap();
        assert_eq!(
            source.source_text(&path).unwrap().text,
            back,
            "shape {n}, second save"
        );
    }
}
