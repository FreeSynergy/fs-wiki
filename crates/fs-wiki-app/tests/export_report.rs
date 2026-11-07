//! Export says when the file cannot be written exactly, before it is written.
//!
//! Pattern: Integration test over a real in-memory wiki, the reader's data source and a temp folder.

mod common;

use common::{temp_dir, wiki};
use fs_components_workbenches::{DocumentSource, ErrorKind};
use fs_wiki_app::WikiSource;
use fs_wiki_core::Texts;

#[test]
fn an_inexact_page_asks_first_and_writes_only_after_the_answer() {
    let (rt, w) = wiki();
    let entry = rt.block_on(w.create("p.md", "last\n")).unwrap();
    let mut doc = entry.document.clone();
    let at = doc.blocks.len() - 1;
    let n = doc.blocks[at].text.len();
    doc.blocks[at].splice(0, n, " leading blank");
    rt.block_on(w.edit(entry.id, doc)).unwrap();
    let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![]);
    let dir = temp_dir("er-inexact");
    let target = dir.display().to_string();

    let err = src.export("p.md", &target, false).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict);
    assert!(
        err.message.contains("could not be written exactly"),
        "{}",
        err.message
    );
    assert!(
        !dir.join("p.md").exists(),
        "nothing is written before the answer"
    );
    // "Yes" without a question just asked does not count either.
    assert!(
        src.export("p.md", &dir.join("other").display().to_string(), true)
            .is_err()
    );
    assert!(!dir.join("other").join("p.md").exists());
    // Asked anew (another export in between clears the question), then answered.
    assert!(src.export("p.md", &target, false).is_err());
    src.export("p.md", &target, true).unwrap();
    assert!(dir.join("p.md").exists());
}

#[test]
fn an_exact_page_is_written_at_once() {
    let (rt, w) = wiki();
    rt.block_on(w.create("p.md", "clean text\n")).unwrap();
    let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![]);
    let dir = temp_dir("er-exact");
    src.export("p.md", &dir.display().to_string(), false)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("p.md")).unwrap(),
        "clean text\n"
    );
}

#[test]
fn one_question_names_both_the_inexactness_and_the_file_that_would_be_replaced() {
    let (rt, w) = wiki();
    let entry = rt.block_on(w.create("p.md", "last\n")).unwrap();
    let mut doc = entry.document.clone();
    let at = doc.blocks.len() - 1;
    let n = doc.blocks[at].text.len();
    doc.blocks[at].splice(0, n, " leading blank");
    rt.block_on(w.edit(entry.id, doc)).unwrap();
    let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![]);
    let dir = temp_dir("er-both");
    std::fs::write(dir.join("p.md"), "somebody else's file\n").unwrap();
    let target = dir.display().to_string();
    let err = src.export("p.md", &target, false).unwrap_err();
    assert!(
        err.message.contains("could not be written exactly"),
        "{}",
        err.message
    );
    assert!(
        err.message.contains("already exists with other content"),
        "{}",
        err.message
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("p.md")).unwrap(),
        "somebody else's file\n"
    );
    src.export("p.md", &target, true).unwrap();
    assert!(
        std::fs::read_to_string(dir.join("p.md"))
            .unwrap()
            .contains("leading blank")
    );
}
