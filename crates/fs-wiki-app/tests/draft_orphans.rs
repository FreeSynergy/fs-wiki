//! A draft of a page that moved or was deleted is found, not lost.
//!
//! Pattern: Example-based test of the draft store.

mod common;

use common::temp_dir;
use fs_components_workbenches::{Draft, DraftStore};
use fs_wiki_app::FileDraftStore;

fn draft(path: &str, text: &str) -> Draft {
    Draft::new(path, text, "orig", Some("rev1".into()))
}

#[test]
fn a_draft_of_a_page_that_moved_is_found_as_an_orphan() {
    let (docs, base) = (temp_dir("dr-orphan-docs"), temp_dir("dr-orphan"));
    let store = FileDraftStore::new(base, &docs).unwrap();
    store.save(&draft("old/place.md", "unsaved words")).unwrap();
    store.save(&draft("still/here.md", "other")).unwrap();
    let orphans = store.orphans(&|p| p == "still/here.md").unwrap();
    assert_eq!(orphans.len(), 1);
    assert_eq!(
        (orphans[0].path.as_str(), orphans[0].text.as_str()),
        ("old/place.md", "unsaved words")
    );
}
