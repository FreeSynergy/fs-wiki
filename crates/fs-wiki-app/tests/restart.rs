//! A restart with a store file and a real folder: what the database holds is the truth (ADR 0042).
//!
//! Pattern: Integration tests over a file store and a temp folder, "restart" = open the store again.

mod common;

use common::{markdown_at, save, temp_dir, wiki_in_file, write_files};
use fs_wiki_app::import_folder;
use fs_wiki_core::{Paging, Texts};

fn paths(rt: &tokio::runtime::Runtime, w: &fs_wiki_app::Wiki) -> Vec<String> {
    let all = rt.block_on(w.list("", Paging::default())).unwrap();
    all.into_iter().map(|t| t.path).collect()
}

#[test]
fn a_save_survives_the_restart() {
    let dir = temp_dir("rs-save");
    let store = dir.join("store.db");
    let docs = dir.join("docs");
    write_files(&docs, &[("p.md", "old text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        save(&rt, &w, "p.md", "SAVED text");
    }
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(import_folder(&**w, &docs));
    assert_eq!(markdown_at(&rt, &w, "p.md").unwrap().trim(), "SAVED text");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_moved_folder_does_not_come_back() {
    let dir = temp_dir("rs-move");
    let store = dir.join("store.db");
    let docs = dir.join("docs");
    write_files(&docs, &[("a/x.md", "# X\n"), ("a/y.md", "# Y\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        rt.block_on(w.move_folder("a", "moved")).unwrap();
    }
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(import_folder(&**w, &docs));
    assert_eq!(paths(&rt, &w), ["moved/x.md", "moved/y.md"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_outside_change_of_an_unchanged_text_is_an_edit() {
    let dir = temp_dir("rs-outside");
    let store = dir.join("store.db");
    let docs = dir.join("docs");
    write_files(&docs, &[("p.md", "old text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    write_files(&docs, &[("p.md", "outside text\n")]);
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(import_folder(&**w, &docs));
    assert_eq!(markdown_at(&rt, &w, "p.md").unwrap().trim(), "outside text");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn both_changed_keeps_the_database_text() {
    let dir = temp_dir("rs-both");
    let store = dir.join("store.db");
    let docs = dir.join("docs");
    write_files(&docs, &[("p.md", "old text\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        save(&rt, &w, "p.md", "SAVED text");
    }
    write_files(&docs, &[("p.md", "outside text\n")]);
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(import_folder(&**w, &docs));
    assert_eq!(markdown_at(&rt, &w, "p.md").unwrap().trim(), "SAVED text");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_deleted_text_stays_deleted_and_a_deleted_file_keeps_its_text() {
    let dir = temp_dir("rs-delete");
    let store = dir.join("store.db");
    let docs = dir.join("docs");
    write_files(&docs, &[("p.md", "# P\n"), ("q.md", "# Q\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let p = rt.block_on(w.get_by_path("p.md")).unwrap().unwrap();
        rt.block_on(w.delete(p.id)).unwrap();
    }
    std::fs::remove_file(docs.join("q.md")).unwrap();
    let (rt, w) = wiki_in_file(&store);
    rt.block_on(import_folder(&**w, &docs));
    assert_eq!(paths(&rt, &w), ["q.md"]);
    std::fs::remove_dir_all(dir).unwrap();
}
