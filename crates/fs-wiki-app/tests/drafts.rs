//! The file store for drafts: round trip, atomic write, damaged file, refused places.
//!
//! Pattern: Example-based tests of an Adapter.

mod common;

use common::temp_dir;
use fs_components_workbenches::{Draft, DraftStore};
use fs_wiki_app::FileDraftStore;

/// The one subfolder (per documentation source) a store made below `base`.
fn only_subfolder(base: &std::path::Path) -> std::path::PathBuf {
    let mut dirs: Vec<_> = std::fs::read_dir(base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs.pop().unwrap()
}

fn draft(path: &str, text: &str) -> Draft {
    Draft::new(path, text, "orig", Some("rev1".into()))
}

#[test]
fn saves_loads_and_clears() {
    let (docs, dir) = (temp_dir("dr-docs"), temp_dir("dr-dir"));
    let store = FileDraftStore::new(dir.join("drafts"), &docs).unwrap();
    assert_eq!(store.load("a/b.md").unwrap(), None);
    store.save(&draft("a/b.md", "first")).unwrap();
    store.save(&draft("a/b.md", "second\nline")).unwrap();
    let d = store.load("a/b.md").unwrap().unwrap();
    assert_eq!(
        (d.text.as_str(), d.original.as_str(), d.revision.as_deref()),
        ("second\nline", "orig", Some("rev1"))
    );
    assert_eq!(
        store.load("a/c.md").unwrap(),
        None,
        "another document has no draft"
    );
    store.clear("a/b.md").unwrap();
    store.clear("a/b.md").unwrap();
    assert_eq!(store.load("a/b.md").unwrap(), None);
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_write_is_atomic_and_leaves_no_temporary_file() {
    let (docs, base) = (temp_dir("dr-atomic-docs"), temp_dir("dr-atomic"));
    let store = FileDraftStore::new(base.clone(), &docs).unwrap();
    for i in 0..5 {
        store.save(&draft("p.md", &format!("v{i}"))).unwrap();
    }
    let dir = only_subfolder(&base);
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with(".draft"));
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_damaged_backup_is_an_error_not_a_silent_nothing() {
    let (docs, base) = (temp_dir("dr-bad-docs"), temp_dir("dr-bad"));
    let store = FileDraftStore::new(base.clone(), &docs).unwrap();
    store.save(&draft("p.md", "x")).unwrap();
    let dir = only_subfolder(&base);
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(file, "{ not json").unwrap();
    let err = store.load("p.md").unwrap_err();
    assert!(err.message.contains("damaged"), "{err}");
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_store_inside_the_docs_folder_is_refused() {
    let docs = temp_dir("dr-inside");
    assert!(FileDraftStore::new(docs.join("drafts"), &docs).is_err());
    assert!(FileDraftStore::new(docs.clone(), &docs).is_err());
    assert!(FileDraftStore::new(docs.join("a/b/c"), &docs).is_err());
    std::fs::remove_dir_all(docs).unwrap();
}

#[cfg(unix)]
#[test]
fn our_folder_and_files_are_private_and_a_given_folder_is_not_changed() {
    use std::os::unix::fs::PermissionsExt;
    let (docs, base) = (temp_dir("dr-perm-docs"), temp_dir("dr-perm"));
    std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
    let store = FileDraftStore::new(base.clone(), &docs).unwrap();
    store.save(&draft("p.md", "private")).unwrap();
    let sub = only_subfolder(&base);
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode(&base),
        0o755,
        "a folder we did not make is left as it was"
    );
    assert_eq!(mode(&sub), 0o700);
    let file = std::fs::read_dir(&sub)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(mode(&file), 0o600);
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_very_long_path_has_a_backup_too() {
    let (docs, base) = (temp_dir("dr-long-docs"), temp_dir("dr-long"));
    let store = FileDraftStore::new(base.clone(), &docs).unwrap();
    let long = format!("{}/x.md", "very-long-folder-name/".repeat(12));
    assert!(long.len() > 200);
    store.save(&draft(&long, "text of the long one")).unwrap();
    store.save(&draft(&format!("{long}2"), "another")).unwrap();
    assert_eq!(
        store.load(&long).unwrap().unwrap().text,
        "text of the long one"
    );
    assert_eq!(store.load(&long).unwrap().unwrap().path, long);
    store.clear(&long).unwrap();
    assert_eq!(store.load(&long).unwrap(), None);
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn two_wikis_in_one_folder_do_not_see_each_others_drafts() {
    let (docs_a, docs_b, base) = (temp_dir("dr-wa"), temp_dir("dr-wb"), temp_dir("dr-shared"));
    let a = FileDraftStore::new(base.clone(), &docs_a).unwrap();
    let b = FileDraftStore::new(base.clone(), &docs_b).unwrap();
    a.save(&draft("index.md", "wiki A's draft")).unwrap();
    assert_eq!(b.load("index.md").unwrap(), None);
    assert!(a.load("index.md").unwrap().is_some());
    for d in [docs_a, docs_b, base] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn concurrent_saves_of_one_document_leave_one_whole_file() {
    let (docs, base) = (temp_dir("dr-conc-docs"), temp_dir("dr-conc"));
    let store = std::sync::Arc::new(FileDraftStore::new(base.clone(), &docs).unwrap());
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let s = store.clone();
            std::thread::spawn(move || {
                for n in 0..20 {
                    s.save(&draft("p.md", &format!("writer {i} version {n}")))
                        .unwrap();
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert!(
        store
            .load("p.md")
            .unwrap()
            .unwrap()
            .text
            .starts_with("writer ")
    );
    let sub = only_subfolder(&base);
    assert_eq!(
        std::fs::read_dir(sub).unwrap().count(),
        1,
        "temporary files were left"
    );
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(base).unwrap();
}
