//! More of the menu Page against the real wiki: index pages, the name index, links that follow a rename.
//!
//! Pattern: Integration test over the public API, no window is drawn (no graphics needed); the dialogs are answered through
//! the slot of the modal, which calls what the buttons and keys call.

mod page_rig;

use fs_wiki_app::frontend_window::{PAGE_DELETE, PAGE_NEW, PAGE_RENAME};
use page_rig::rig;

#[test]
fn a_page_called_reindex_is_no_index_page_it_renames_and_deletes_with_its_subpages() {
    let r = rig(false);
    r.make("reindex.md", "# Reindex\n");
    r.make("reindex/part.md", "# Part\n");
    r.open("reindex.md");
    assert!(r.app.choose(PAGE_RENAME));
    assert!(r.modal.submit("Reindexed"));
    assert!(r.paths().contains(&"reindexed.md".to_owned()));
    assert!(
        r.paths().contains(&"reindexed/part.md".to_owned()),
        "{:?}",
        r.paths()
    );
    r.open("reindexed.md");
    assert!(r.app.choose(PAGE_DELETE));
    assert!(r.modal.yes());
    assert!(
        !r.paths().iter().any(|p| p.starts_with("reindexed")),
        "the subpage went along: {:?}",
        r.paths()
    );
    let trash = r.rt.block_on(r.wiki.list_trash(None, 10)).unwrap();
    assert_eq!(trash.items.len(), 2);
}

#[test]
fn the_name_index_is_refused_and_main_index_is_fine() {
    let r = rig(false);
    assert!(r.app.choose(PAGE_NEW));
    assert!(r.modal.submit("Index"), "refused: the dialog stays");
    assert_eq!(r.paths().len(), 3);
    assert!(r.modal.submit("Main index"));
    assert!(r.paths().contains(&"main-index.md".to_owned()));
}

#[test]
fn links_to_a_renamed_page_and_to_its_subpage_follow() {
    let r = rig(false);
    r.make(
        "links.md",
        "# Links\n\n[one](handbook.md) and [two](handbook/chapter-one.md)\n",
    );
    r.open("handbook.md");
    assert!(r.app.choose(PAGE_RENAME));
    assert!(r.modal.submit("Manual"));
    let text = r.text("links.md");
    assert!(
        text.contains("](manual.md)") && text.contains("](manual/chapter-one.md)"),
        "{text}"
    );
    assert!(!text.contains("handbook"), "{text}");
}
