//! Comments are not displayed by the reader, but the editor and the export keep them.
//!
//! Pattern: Integration test over a real in-memory wiki and the reader's data source.

mod common;

use common::wiki;
use fs_components_workbenches::DocumentSource;
use fs_wiki_app::WikiSource;
use fs_wiki_core::Texts;

const PAGE: &str = "Shown words.\n\n[comment]: # (hush-one)\n\n<!-- hush-two -->\n\n[//]: <> (hush-three)\n\nMore shown.\n";

#[test]
fn not_displayed_in_the_reader_nor_found_but_present_in_the_editor() {
    let (rt, w) = wiki();
    rt.block_on(w.create("p.md", PAGE)).unwrap();
    let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![]);
    let shown = format!("{:?}", src.read("p.md").unwrap().blocks);
    assert!(shown.contains("Shown words") && shown.contains("More shown"));
    for word in ["hush-one", "hush-two", "hush-three"] {
        assert!(!shown.contains(word), "{word} is displayed");
        assert!(src.find(word).unwrap().is_empty(), "{word} is found");
    }
    let editor = src.source_text("p.md").unwrap().text;
    for word in ["hush-one", "hush-two", "hush-three"] {
        assert!(editor.contains(word), "{word} is missing in the editor");
    }
}
