//! What a sync reports instead of overwriting, deleting or hiding.
//!
//! Pattern: Integration tests over a file store and a temp folder.

mod common;

use common::{markdown_at, save, temp_dir, wiki_in_file, write_files};
use fs_wiki_app::import_folder;
use fs_wiki_core::{Origins, Texts};

#[test]
fn both_changed_is_a_conflict_with_the_file_version_and_it_stays_one() {
    let dir = temp_dir("cf-both");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("p.md", "old text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        save(&rt, &w, "p.md", "SAVED text");
    }
    write_files(&docs, &[("p.md", "outside text\n")]);
    for _ in 0..2 {
        let (rt, w) = wiki_in_file(&store);
        let report = rt.block_on(import_folder(&**w, &docs));
        assert_eq!(report.conflicts.len(), 1);
        let c = &report.conflicts[0];
        assert_eq!(
            (c.file.as_str(), c.text_path.as_deref()),
            ("p.md", Some("p.md"))
        );
        assert_eq!(
            c.file_text, "outside text\n",
            "the file's version is not lost"
        );
        assert_eq!(markdown_at(&rt, &w, "p.md").unwrap().trim(), "SAVED text");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_outside_change_reaches_a_text_that_moved_and_a_deleted_text_is_a_conflict() {
    let dir = temp_dir("cf-moved");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("a/x.md", "x\n"), ("a/y.md", "y\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        rt.block_on(w.move_folder("a", "moved")).unwrap();
        let y = rt.block_on(w.get_by_path("moved/y.md")).unwrap().unwrap();
        rt.block_on(w.delete(y.id)).unwrap();
    }
    write_files(
        &docs,
        &[("a/x.md", "x changed\n"), ("a/y.md", "y changed\n")],
    );
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(report.updated, 1);
    assert_eq!(
        markdown_at(&rt, &w, "moved/x.md").unwrap().trim(),
        "x changed"
    );
    assert!(markdown_at(&rt, &w, "a/x.md").is_none());
    assert!(
        markdown_at(&rt, &w, "moved/y.md").is_none(),
        "stays deleted"
    );
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].text_path, None);
    assert_eq!(report.conflicts[0].file_text, "y changed\n");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_gone_file_is_reported_and_its_text_kept() {
    let dir = temp_dir("cf-gone");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("p.md", "p\n"), ("q.md", "q\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    std::fs::remove_file(docs.join("q.md")).unwrap();
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(report.gone.len(), 1);
    assert_eq!(report.gone[0].path, "q.md");
    assert_eq!(report.unchanged, 1);
    assert!(markdown_at(&rt, &w, "q.md").is_some());
    assert_eq!(
        rt.block_on(w.origins()).unwrap().len(),
        2,
        "the record stays"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_text_without_a_record_that_differs_from_the_file_is_never_overwritten() {
    let dir = temp_dir("cf-legacy");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("p.md", "file text\n")]);
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(w.create("p.md", "db text\n")).unwrap();
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(markdown_at(&rt, &w, "p.md").unwrap().trim(), "db text");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn nothing_unchanged_is_parsed_again_and_what_is_not_looked_at_is_listed() {
    let dir = temp_dir("cf-skip");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(
        &docs,
        &[("p.md", "p\n"), ("UPPER.MD", "x\n"), ("real.md", "r\n")],
    );
    std::os::unix::fs::symlink(docs.join("real.md"), docs.join("link.md")).unwrap();
    std::os::unix::fs::symlink(&docs, docs.join("loop")).unwrap();
    let (rt, w) = wiki_in_file(&store);
    let first = rt.block_on(import_folder(&**w, &docs));
    let mut skipped: Vec<_> = first.skipped.iter().map(|s| s.path.as_str()).collect();
    skipped.sort_unstable();
    assert_eq!(skipped, ["UPPER.MD", "link.md", "loop"]);
    let before = rt
        .block_on(w.list("", fs_wiki_core::Paging::default()))
        .unwrap();
    let second = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (second.imported, second.updated, second.unchanged),
        (0, 0, 2)
    );
    let after = rt
        .block_on(w.list("", fs_wiki_core::Paging::default()))
        .unwrap();
    assert_eq!(before, after, "not one text was written");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_emptied_outside_is_a_conflict_not_an_edit_that_empties_the_text() {
    let dir = temp_dir("cf-empty");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("p.md", "important text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    write_files(&docs, &[("p.md", "")]);
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!((report.updated, report.conflicts.len()), (0, 1));
    assert_eq!(
        markdown_at(&rt, &w, "p.md").unwrap().trim(),
        "important text"
    );
}

#[test]
fn a_file_with_only_a_byte_order_mark_counts_as_empty() {
    let dir = temp_dir("cf-bom");
    let (store, docs) = (dir.join("store.db"), dir.join("docs"));
    write_files(&docs, &[("p.md", "important text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    write_files(&docs, &[("p.md", "\u{feff}\n")]);
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!((report.updated, report.conflicts.len()), (0, 1));
    assert_eq!(
        markdown_at(&rt, &w, "p.md").unwrap().trim(),
        "important text"
    );
}
