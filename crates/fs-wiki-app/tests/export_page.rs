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
        rt.block_on(import_folder(w.as_ref(), &docs));
        rt.block_on(w.move_folder("a", "moved")).unwrap();
        let src = WikiSource::new(w.clone(), rt.handle().clone(), vec![])
            .with_origins(w.clone(), docs.clone());
        let msg = src
            .export("moved/x.md", &docs.display().to_string(), false)
            .unwrap();
        assert!(msg.contains("a/x.md"), "{msg}");
        assert!(docs.join("a/x.md").exists(), "never deleted");
        assert!(docs.join("moved/x.md").exists());
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(w.as_ref(), &docs));
    assert_eq!(
        (report.imported, report.updated, report.unchanged),
        (0, 0, 2)
    );
    assert!(report.conflicts.is_empty());
    let all = rt
        .block_on(w.list("", fs_wiki_core::Paging::default()))
        .unwrap();
    assert_eq!(all.len(), 1, "no second copy of the page");
    std::fs::remove_dir_all(dir).unwrap();
}
