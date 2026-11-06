//! The file store for drafts: round trip, atomic write, damaged file, refused places.
//!
//! Pattern: Example-based tests of an Adapter.

mod common;

use common::temp_dir;
use fs_components_workbenches::{Draft, DraftStore};
use fs_wiki_app::FileDraftStore;

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
    let (docs, dir) = (temp_dir("dr-atomic-docs"), temp_dir("dr-atomic"));
    let store = FileDraftStore::new(dir.clone(), &docs).unwrap();
    for i in 0..5 {
        store.save(&draft("p.md", &format!("v{i}"))).unwrap();
    }
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with(".draft"));
    std::fs::remove_dir_all(docs).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_damaged_backup_is_an_error_not_a_silent_nothing() {
    let (docs, dir) = (temp_dir("dr-bad-docs"), temp_dir("dr-bad"));
    let store = FileDraftStore::new(dir.clone(), &docs).unwrap();
    store.save(&draft("p.md", "x")).unwrap();
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
