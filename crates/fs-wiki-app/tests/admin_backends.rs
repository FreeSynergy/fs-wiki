//! The backends of the Admin frontend against the real server: lists, commands, partial change, the unsupported answer.
//!
//! Pattern: Integration tests against the `Backend` trait of `fs-admin-frontend`; no stand-ins.

mod admin_common;

use admin_common::{backend, create, set, v, wiki};
use fs_admin_frontend::{BackendError, Values};
use fs_types::{BasicType, Edit};

#[test]
fn a_page_is_created_listed_changed_partly_and_found() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "manual/start", "# Start\n\nHello\n");
    let all = pages.list(&Values::new()).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].values["path"], ["manual/start"]);
    assert!(all[0].values["content"][0].contains("Hello"));
    // nothing is in the edit: the content stays (a field not sent is unchanged)
    pages.change(&id, &Edit::new()).unwrap();
    assert!(pages.list(&Values::new()).unwrap()[0].values["content"][0].contains("Hello"));
    pages
        .change(&id, &set("content", &["# Start\n\nChanged\n"]))
        .unwrap();
    let changed = pages.list(&v(&[("term", &["changed"])])).unwrap();
    assert_eq!(changed.len(), 1);
    let hits = pages
        .list(&v(&[("term", &["changed"]), ("pathPrefix", &["manual"])]))
        .unwrap();
    assert_eq!(hits.len(), 1, "a search with a filter");
    let none = pages
        .list(&v(&[("term", &["changed"]), ("pathPrefix", &["other"])]))
        .unwrap();
    assert_eq!(none.len(), 0);
}

#[test]
fn an_edit_names_what_changes_and_the_rest_stays() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "a", "one\n");
    pages
        .change(
            &id,
            &set("redirectTo", &["b"])
                .set(&["kind"], BasicType::Choice, &["template"])
                .set(&["path"], BasicType::Text, &["c"]),
        )
        .unwrap();
    pages.list(&Values::new()).unwrap();
    let page = &pages.list(&Values::new()).unwrap()[0];
    assert_eq!(page.values["redirectTo"], ["b"]);
    assert_eq!(page.values["kind"], ["template"]);
    assert_eq!(page.values["path"], ["c"]);
    assert!(
        page.values["content"][0].contains("one"),
        "the content stayed"
    );
    // no value clears: the redirect is gone, the content is not touched
    pages
        .change(&id, &Edit::new().clear(&["redirectTo"], BasicType::Text))
        .unwrap();
    assert!(
        !pages.list(&Values::new()).unwrap()[0]
            .values
            .contains_key("redirectTo")
    );
    assert_eq!(
        pages.change("not-an-id", &Edit::new()).unwrap_err(),
        BackendError::NotFound
    );
    let e = pages
        .run("create", None, &v(&[("content", &["no path"])]))
        .unwrap_err();
    assert!(
        matches!(e, BackendError::Rejected(_)),
        "a required field is a finding: {e:?}"
    );
}

#[test]
fn a_command_that_is_described_but_not_carried_out_says_unsupported() {
    let w = wiki();
    for (ty, cmd) in fs_wiki_core::describe::unsupported() {
        assert!(!backend(&w, &ty).supported(&cmd), "{ty}.{cmd} is locked");
        let e = backend(&w, &ty)
            .run(&cmd, None, &Values::new())
            .unwrap_err();
        assert_eq!(
            e,
            BackendError::Unavailable(format!("unsupported:{cmd}")),
            "{ty}.{cmd}"
        );
    }
    let e = backend(&w, "Page")
        .list(&v(&[("tag", &["x"])]))
        .unwrap_err();
    assert_eq!(e, BackendError::Unavailable("unsupported:tagTable".into()));
}

#[test]
fn what_the_core_carries_out_is_unlocked_for_the_frontend() {
    let w = wiki();
    for (ty, cmd) in fs_wiki_core::describe::SUPPORTED {
        assert!(backend(&w, ty).supported(cmd), "{ty}.{cmd} is unlocked");
    }
}

#[test]
fn trash_template_redirect_resolve_and_export() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "t", "# T\n");
    pages
        .run(
            "setTemplate",
            None,
            &v(&[("id", &[&id]), ("kind", &["template"])]),
        )
        .unwrap();
    assert_eq!(
        pages.list(&Values::new()).unwrap()[0].values["kind"],
        ["template"]
    );
    let copy = pages
        .run(
            "createFromTemplate",
            None,
            &v(&[("template", &[&id]), ("path", &["t2"])]),
        )
        .unwrap()
        .id
        .unwrap();
    pages
        .run(
            "setRedirect",
            None,
            &v(&[("id", &[&copy]), ("redirectTo", &["t"])]),
        )
        .unwrap();
    let r = pages
        .run("resolve", None, &v(&[("path", &["t2"])]))
        .unwrap();
    assert_eq!(r.values["outcome"], ["resolved"]);
    assert_eq!(r.id.as_deref(), Some(id.as_str()));
    let exported = pages
        .run(
            "export",
            None,
            &v(&[("id", &[&id]), ("mediaType", &["text/markdown"])]),
        )
        .unwrap();
    assert!(exported.values["text"][0].contains('T'));
    pages.run("trash", None, &v(&[("id", &[&copy])])).unwrap();
    let trash = backend(&w, "TrashEntry");
    let entries = trash.list(&Values::new()).unwrap();
    assert_eq!(entries.len(), 1);
    trash.run("restore", None, &v(&[("id", &[&copy])])).unwrap();
    assert_eq!(trash.list(&Values::new()).unwrap().len(), 0);
    assert_eq!(pages.list(&Values::new()).unwrap().len(), 2);
}

#[test]
fn folders_revisions_and_links() {
    let w = wiki();
    let folders = backend(&w, "Folder");
    let space = folders
        .run("create", None, &v(&[("name", &["Manual"])]))
        .unwrap()
        .id
        .unwrap();
    let sub = folders
        .run(
            "create",
            None,
            &v(&[("parent", &[&space]), ("name", &["Start"])]),
        )
        .unwrap()
        .id
        .unwrap();
    folders
        .run("rename", None, &v(&[("id", &[&sub]), ("name", &["Begin"])]))
        .unwrap();
    let listed = folders.list(&Values::new()).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|r| r.values["name"] == ["Begin"]));
    assert_eq!(folders.list(&v(&[("name", &["begin"])])).unwrap().len(), 1);
    folders
        .run(
            "reorder",
            None,
            &v(&[("id", &[&sub]), ("placement", &["first"])]),
        )
        .unwrap();
    let page = create(&w, "manual/p", "see [x](manual/q)\n");
    let revs = backend(&w, "Revision");
    let list = revs.list(&v(&[("page", &[&page])])).unwrap();
    assert_eq!(list.len(), 1);
    let got = revs
        .run("get", None, &v(&[("id", &[&list[0].id])]))
        .unwrap();
    assert!(got.values["content"][0].contains("see"));
    assert!(
        revs.list(&Values::new()).unwrap().is_empty(),
        "history is per page"
    );
    let links = backend(&w, "Link")
        .list(&v(&[("source", &[&page])]))
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].values["target"], ["manual/q"]);
    folders.run("delete", None, &v(&[("id", &[&sub])])).unwrap();
    assert_eq!(folders.list(&Values::new()).unwrap().len(), 1);
}
