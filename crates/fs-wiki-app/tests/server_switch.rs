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
fn a_text_deleted_in_the_app_stays_deleted_and_is_not_reported() {
    let dir = temp_dir("sw-deleted");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n"), ("b.md", "# B\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let b = rt.block_on(w.get_by_path("b.md")).unwrap().unwrap();
        rt.block_on(w.delete(b.id)).unwrap();
    }
    for _ in 0..2 {
        let (rt, w) = wiki_in_file(&store);
        let report = rt.block_on(import_folder(&**w, &docs));
        assert_eq!(
            (report.imported, report.unchanged, report.records_ignored),
            (0, 2, 0)
        );
        assert!(
            report.missing.is_empty(),
            "the person knows: {:?}",
            report.missing
        );
        assert!(rt.block_on(w.get_by_path("b.md")).unwrap().is_none());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_text_deleted_outside_the_app_is_listed_every_start_and_not_brought_back() {
    let dir = temp_dir("sw-outside");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n"), ("b.md", "# B\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let b = rt.block_on(w.get_by_path("b.md")).unwrap().unwrap();
        // Another program deletes it on the server: not through the app's `Wiki`.
        rt.block_on(w.texts().delete(b.id)).unwrap();
    }
    for _ in 0..2 {
        let (rt, w) = wiki_in_file(&store);
        let report = rt.block_on(import_folder(&**w, &docs));
        assert_eq!((report.imported, report.unchanged), (0, 1));
        let missing: Vec<_> = report.missing.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(missing, ["b.md"], "listed, not counted as unchanged");
        assert!(rt.block_on(w.get_by_path("b.md")).unwrap().is_none());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_server_restored_from_an_older_copy_lists_what_it_lacks_and_does_not_import_it() {
    let dir = temp_dir("sw-backup");
    let (docs, store, backup) = (
        dir.join("docs"),
        dir.join("store.db"),
        dir.join("backup.db"),
    );
    write_files(&docs, &[("a.md", "# A\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    // The copy of the server taken now; the records follow later.
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = e.file_name().into_string().unwrap();
        if name.starts_with("store.db") && !name.contains(".origins") {
            std::fs::copy(
                e.path(),
                dir.join(name.replacen("store.db", "backup.db", 1)),
            )
            .unwrap();
        }
    }
    write_files(&docs, &[("b.md", "# B\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        assert_eq!(rt.block_on(import_folder(&**w, &docs)).imported, 1);
    }
    std::fs::copy(
        store.with_extension("origins"),
        backup.with_extension("origins"),
    )
    .unwrap();
    let (rt, w) = wiki_in_file(&backup);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.unchanged, report.records_ignored),
        (0, 1, 0)
    );
    let missing: Vec<_> = report.missing.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(missing, ["b.md"]);
    assert!(report.has_problems());
    assert!(
        rt.block_on(w.get_by_path("b.md")).unwrap().is_none(),
        "not brought back by itself"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn when_every_text_of_a_folder_was_deleted_outside_the_server_is_still_the_same_and_they_are_listed()
 {
    // What the store's identity settles: the server holds none of the texts the records name, and it
    // is still the same server, so nothing is imported again behind the person's back.
    let dir = temp_dir("sw-all");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let a = rt.block_on(w.get_by_path("a.md")).unwrap().unwrap();
        rt.block_on(w.texts().delete(a.id)).unwrap();
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!((report.imported, report.records_ignored), (0, 0));
    let missing: Vec<_> = report.missing.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(missing, ["a.md"]);
    assert!(rt.block_on(w.get_by_path("a.md")).unwrap().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

/// The file as an older version wrote it: records only, no line for a store.
fn strip_headers(records: &std::path::Path) {
    let text = std::fs::read_to_string(records).unwrap();
    let old: String = text
        .lines()
        .filter(|l| !l.contains("\"store\""))
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(old.len() < text.len(), "there was a header to strip");
    std::fs::write(records, old).unwrap();
}

#[test]
fn records_from_before_the_store_was_recorded_are_taken_over_when_the_server_has_their_texts() {
    let dir = temp_dir("sw-legacy");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
    }
    strip_headers(&store.with_extension("origins"));
    {
        let (rt, w) = wiki_in_file(&store);
        let report = rt.block_on(import_folder(&**w, &docs));
        assert_eq!(
            (report.imported, report.unchanged, report.records_ignored),
            (0, 1, 0)
        );
    }
    let text = std::fs::read_to_string(store.with_extension("origins")).unwrap();
    assert!(
        text.lines().next().unwrap().contains("\"store\""),
        "now bound: {text}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn records_from_before_the_store_was_recorded_are_not_bound_to_a_server_that_lacks_their_texts() {
    let dir = temp_dir("sw-legacy-fresh");
    let (docs, records) = (dir.join("docs"), dir.join("o.origins"));
    write_files(&docs, &[("a.md", "# A\n")]);
    {
        let (rt, w) = wiki_with_records(&records);
        rt.block_on(import_folder(&**w, &docs));
    }
    strip_headers(&records);
    let (rt, w) = wiki_with_records(&records);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.records_ignored),
        (1, 1),
        "another server: the file comes in, the old record is not trusted"
    );
    assert!(report.missing.is_empty());
    let text = std::fs::read_to_string(&records).unwrap();
    let first = text.lines().next().unwrap();
    assert!(
        !first.contains("\"store\""),
        "the old records were not claimed by this server: {text}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn going_back_to_a_server_brings_its_records_and_deleted_marks_back() {
    let dir = temp_dir("sw-aba");
    let (docs, store) = (dir.join("docs"), dir.join("a.db"));
    let records = store.with_extension("origins");
    write_files(&docs, &[("a.md", "# A\n"), ("b.md", "# B\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let b = rt.block_on(w.get_by_path("b.md")).unwrap().unwrap();
        rt.block_on(w.delete(b.id)).unwrap();
    }
    {
        let (rt, w) = wiki_with_records(&records);
        let report = rt.block_on(import_folder(&**w, &docs));
        assert_eq!((report.imported, report.records_ignored), (2, 2));
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.unchanged, report.records_ignored),
        (0, 2, 0),
        "b.md stays deleted: its mark came back with the records of this server"
    );
    assert!(report.missing.is_empty());
    assert!(rt.block_on(w.get_by_path("b.md")).unwrap().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn when_every_text_of_a_folder_was_deleted_in_the_app_they_stay_deleted() {
    let dir = temp_dir("sw-all-app");
    let (docs, store) = (dir.join("docs"), dir.join("store.db"));
    write_files(&docs, &[("a.md", "# A\n")]);
    {
        let (rt, w) = wiki_in_file(&store);
        rt.block_on(import_folder(&**w, &docs));
        let a = rt.block_on(w.get_by_path("a.md")).unwrap().unwrap();
        rt.block_on(w.delete(a.id)).unwrap();
    }
    let (rt, w) = wiki_in_file(&store);
    let report = rt.block_on(import_folder(&**w, &docs));
    assert_eq!(
        (report.imported, report.unchanged, report.records_ignored),
        (0, 1, 0)
    );
    assert!(rt.block_on(w.get_by_path("a.md")).unwrap().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
