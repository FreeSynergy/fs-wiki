//! The records of imported files belong to one wiki. Another server must get the files again; a
//! text deleted on purpose in the same wiki must not come back.
//!
//! Pattern: Integration tests over two real servers, one records file and a real folder.

mod common;

use common::{temp_dir, wiki_in_file, wiki_with_records, write_files};
use fs_wiki_app::import_folder;
use fs_wiki_core::{Paging, Texts};

#[test]
fn a_fresh_server_with_old_records_gets_every_file_again() {
    let dir = temp_dir("sw-fresh");
    let (docs, records) = (dir.join("docs"), dir.join("o.origins"));
    write_files(&docs, &[("a.md", "# A\n"), ("b/c.md", "# C\n")]);
    {
        let (rt, w) = wiki_with_records(&records);
        let first = rt.block_on(import_folder(&**w, &docs));
        assert_eq!((first.imported, first.records_ignored), (2, 0));
    }
    let (rt, w) = wiki_with_records(&records);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.unchanged, report.records_ignored),
        (2, 0, 2),
        "the old records say nothing about this server"
    );
    let all = rt.block_on(w.list("", Paging::default())).unwrap();
    assert_eq!(all.len(), 2);
    let again = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (again.imported, again.unchanged, again.records_ignored),
        (0, 2, 0)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_text_deleted_on_purpose_stays_deleted_on_the_same_server() {
    let dir = temp_dir("sw-deleted");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n"), ("b.md", "# B\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let b = rt.block_on(w.get_by_path("b.md")).unwrap().unwrap();
        rt.block_on(w.delete(b.id)).unwrap();
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.unchanged, report.records_ignored),
        (0, 2, 0)
    );
    assert!(rt.block_on(w.get_by_path("b.md")).unwrap().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
