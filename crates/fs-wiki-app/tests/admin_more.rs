//! The backends of the Admin frontend against the real server: folders, history, trash, redirects, conflicts.
//!
//! Pattern: Integration tests against the `Backend` trait of `fs-admin-frontend`; no stand-ins.

mod admin_common;

use admin_common::{backend, create, v, wiki};
use fs_admin_frontend::{BackendError, Values};
use fs_wiki_app::admin::views;
use fs_wiki_core::Texts;

#[test]
fn the_window_gets_a_view_per_type_from_the_servers_descriptions() {
    let w = wiki();
    let registry = w.rt.block_on(w.texts.types()).unwrap();
    let views = views(&registry, &w.ctx);
    let ids: Vec<_> = views.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, ["Page", "Folder", "Revision", "TrashEntry", "Link"]);
}

#[test]
fn a_redirect_not_sent_stays_and_an_empty_one_clears_it() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "a", "one\n");
    pages
        .run("setRedirect", &v(&[("id", &[&id]), ("redirectTo", &["b"])]))
        .unwrap();
    // the form sends only what changed: nothing sent, nothing changes
    pages.run("setRedirect", &v(&[("id", &[&id])])).unwrap();
    assert_eq!(
        pages.list(&Values::new()).unwrap()[0].values["redirectTo"],
        ["b"]
    );
    pages
        .run("setRedirect", &v(&[("id", &[&id]), ("redirectTo", &[])]))
        .unwrap();
    assert!(
        !pages.list(&Values::new()).unwrap()[0]
            .values
            .contains_key("redirectTo")
    );
}

#[test]
fn a_page_moves_among_its_siblings_and_a_folder_with_it() {
    let w = wiki();
    let folders = backend(&w, "Folder");
    let space = folders
        .run("create", &v(&[("name", &["Manual"])]))
        .unwrap()
        .id
        .unwrap();
    let a = create(&w, "manual/a", "a\n");
    create(&w, "manual/b", "b\n");
    backend(&w, "Page")
        .run("reorder", &v(&[("id", &[&a]), ("placement", &["last"])]))
        .unwrap();
    let order =
        w.rt.block_on(w.texts.children(Some(space.parse().unwrap()), 0, 10))
            .unwrap()
            .items;
    assert_eq!(order.last().unwrap().id.to_string(), a, "a is last now");
    folders
        .run("move", &v(&[("id", &[&space]), ("path", &["guide"])]))
        .unwrap();
    let pages = backend(&w, "Page").list(&Values::new()).unwrap();
    assert!(
        pages
            .iter()
            .all(|p| p.values["path"][0].starts_with("guide/")),
        "{pages:?}"
    );
}

#[test]
fn restore_a_revision_and_the_purge_that_needs_a_hub() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "a", "one\n");
    pages
        .run("change", &v(&[("id", &[&id]), ("content", &["two\n"])]))
        .unwrap();
    let revs = backend(&w, "Revision");
    let list = revs.list(&v(&[("page", &[&id])])).unwrap();
    assert_eq!(list.len(), 2);
    let first = list.iter().find(|r| r.values["number"] == ["1"]).unwrap();
    revs.run("restore", &v(&[("id", &[&first.id])])).unwrap();
    assert!(pages.list(&Values::new()).unwrap()[0].values["content"][0].contains("one"));
    let e = revs.run("purge", &v(&[("id", &[&first.id])])).unwrap_err();
    assert!(
        matches!(e, BackendError::Unavailable(ref t) if t.contains("no hub")),
        "{e:?}"
    );
    pages.run("trash", &v(&[("id", &[&id])])).unwrap();
    let e = backend(&w, "TrashEntry")
        .run("purge", &v(&[("id", &[&id])]))
        .unwrap_err();
    assert!(
        matches!(e, BackendError::Unavailable(ref t) if t.contains("no hub")),
        "{e:?}"
    );
}

#[test]
fn a_write_between_the_read_and_the_change_is_a_conflict_not_lost() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "a", "one\n");
    let seen = pages.list(&Values::new()).unwrap()[0].values["revision"][0].clone();
    pages
        .run(
            "change",
            &v(&[
                ("id", &[&id]),
                ("content", &["two\n"]),
                ("revision", &[&seen]),
            ]),
        )
        .unwrap();
    // a second writer used the revision that was current when the form was opened
    let e = pages
        .run(
            "change",
            &v(&[
                ("id", &[&id]),
                ("content", &["three\n"]),
                ("revision", &[&seen]),
            ]),
        )
        .unwrap_err();
    assert!(
        matches!(e, BackendError::Unavailable(ref t) if t.contains("conflict")),
        "{e:?}"
    );
    assert!(pages.list(&Values::new()).unwrap()[0].values["content"][0].contains("two"));
}
