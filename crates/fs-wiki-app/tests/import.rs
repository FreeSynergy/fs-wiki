//! Importing a folder: counts, relative paths, failures listed, the folder untouched.
//!
//! Pattern: Integration tests over a real in-memory wiki and a real temp folder.

mod common;

use common::{fingerprint, temp_dir, wiki, write_files};
use fs_wiki_app::import_folder;
use fs_wiki_core::{Paging, Texts};

#[test]
fn imports_with_relative_paths_and_lists_failures_and_leaves_the_folder_alone() {
    let (rt, wiki) = wiki();
    let dir = temp_dir("import");
    write_files(
        &dir,
        &[
            ("index.md", "# Home\n\n[a](a/b.md)\n"),
            ("a/b.md", "# B\n"),
            ("a/c#d.md", "# bad name\n"),
            (".hidden/x.md", "# hidden\n"),
            ("a/notes.txt", "not markdown"),
        ],
    );
    std::fs::write(dir.join("a/latin.md"), [0xff, 0xfe, 0x41]).unwrap();
    let before = fingerprint(&dir);
    let report = rt.block_on(import_folder(wiki.as_ref(), &dir));
    assert_eq!(fingerprint(&dir), before, "the docs folder must not change");
    assert_eq!(report.imported, 2);
    let failed: Vec<&str> = report.failures.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(failed, ["a/c#d.md", "a/latin.md"]);
    assert!(
        report.failures[0].reason.contains('#'),
        "{:?}",
        report.failures[0]
    );
    assert!(
        report.failures[1].reason.contains("read"),
        "{:?}",
        report.failures[1]
    );
    let all = rt.block_on(wiki.list("", Paging::default())).unwrap();
    let paths: Vec<&str> = all.iter().map(|t| t.path.as_str()).collect();
    assert_eq!(paths, ["a/b.md", "index.md"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn importing_again_is_a_no_op() {
    let (rt, wiki) = wiki();
    let dir = temp_dir("again");
    write_files(&dir, &[("p.md", "# P\n\ntext\n")]);
    assert_eq!(rt.block_on(import_folder(wiki.as_ref(), &dir)).imported, 1);
    assert_eq!(rt.block_on(import_folder(wiki.as_ref(), &dir)).imported, 1);
    assert_eq!(
        rt.block_on(wiki.list("", Paging::default())).unwrap().len(),
        1
    );
    std::fs::remove_dir_all(dir).unwrap();
}
