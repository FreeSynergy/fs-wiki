//! Exporting a page into the docs folder after a move: the records stay right, nothing is deleted.
//!
//! Pattern: Integration test over a file store and a temp folder.

mod common;

use common::{temp_dir, wiki_in_file, write_files};
use fs_components_workbenches::DocumentSource;
use fs_wiki_app::{WikiSource, import_folder};
use fs_wiki_core::Texts;

#[test]
fn an_export_after_a_move_names_the_old_file_and_keeps_the_next_start_quiet() {
    let dir = temp_dir("ep-move");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("a/x.md", "# X\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        rt.block_on(w.move_folder("a", "moved")).unwrap();
        let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![])
            .with_origins(w.wiki(), docs.clone());
        let msg = src
            .export("moved/x.md", &docs.display().to_string(), false)
            .unwrap();
        assert!(msg.contains("a/x.md"), "{msg}");
        assert!(docs.join("a/x.md").exists(), "never deleted");
        assert!(docs.join("moved/x.md").exists());
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.updated, report.unchanged),
        (0, 0, 2)
    );
    assert_eq!(report.conflicts, [] as [fs_wiki_app::Conflict; 0]);
    let all = rt
        .block_on(w.list("", fs_wiki_core::Paging::default()))
        .unwrap();
    assert_eq!(all.len(), 1, "no second copy of the page");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_second_press_overwrites_only_the_content_the_person_saw() {
    let dir = temp_dir("ep-bound");
    let docs = dir.join("docs");
    write_files(&docs, &[("p.md", "disk v1\n")]);
    let (rt, w) = common::wiki();
    rt.block_on(w.create("p.md", "wiki text")).unwrap();
    let src = WikiSource::new(w.wiki(), rt.handle().clone(), vec![]);
    let target = docs.display().to_string();
    let disk = || std::fs::read_to_string(docs.join("p.md")).unwrap();

    // Pressing "yes" without having been asked overwrites nothing.
    assert!(src.export("p.md", &target, true).is_err());
    assert_eq!(disk(), "disk v1\n");
    // The question is asked about v1 ...
    assert!(src.export("p.md", &target, false).is_err());
    // ... the file changes before the answer: ask anew, the newer content is not overwritten.
    write_files(&docs, &[("p.md", "disk v2, newer\n")]);
    assert!(src.export("p.md", &target, true).is_err());
    assert_eq!(disk(), "disk v2, newer\n");
    // The answer to the new question counts.
    src.export("p.md", &target, true).unwrap();
    assert_eq!(disk(), "wiki text");
}
