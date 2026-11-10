//! The backends of the Admin frontend against the real server: lists, commands, partial change, the unsupported answer.
//!
//! Pattern: Integration tests against the `Backend` trait of `fs-admin-frontend`; no stand-ins.

mod admin_common;

use admin_common::{backend, create, v, wiki};
use fs_admin_frontend::{BackendError, Values};

#[test]
fn a_page_is_created_listed_changed_partly_and_found() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "manual/start", "# Start\n\nHello\n");
    let all = pages.list(&Values::new()).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].values["path"], ["manual/start"]);
    assert!(all[0].values["content"][0].contains("Hello"));
    // only the summary is sent: the content stays (a field not sent is unchanged)
    pages
        .run("change", &v(&[("id", &[&id]), ("summary", &["note"])]))
        .unwrap();
    assert!(pages.list(&Values::new()).unwrap()[0].values["content"][0].contains("Hello"));
    pages
        .run(
            "change",
            &v(&[
                ("id", &[&id]),
                ("content", &["# Start\n\nChanged\n"]),
                ("summary", &["edit"]),
            ]),
        )
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
    assert!(none.is_empty());
}

#[test]
fn a_stale_revision_is_refused_and_a_bad_number_is_a_finding() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "a", "one\n");
    let e = pages
        .run(
            "change",
            &v(&[
                ("id", &[&id]),
                ("content", &["two\n"]),
                ("revision", &["1"]),
            ]),
        )
        .unwrap_err();
    assert!(
        matches!(e, BackendError::Unavailable(ref t) if t.contains("conflict")),
        "{e:?}"
    );
    let e = pages
        .run("change", &v(&[("id", &[&id]), ("revision", &["x"])]))
        .unwrap_err();
    assert!(matches!(e, BackendError::Rejected(_)), "{e:?}");
    let e = pages
        .run("create", &v(&[("content", &["no path"])]))
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
        let e = backend(&w, &ty).run(&cmd, &Values::new()).unwrap_err();
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
fn trash_template_redirect_resolve_and_export() {
    let w = wiki();
    let pages = backend(&w, "Page");
    let id = create(&w, "t", "# T\n");
    pages
        .run(
            "setTemplate",
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
            &v(&[("template", &[&id]), ("path", &["t2"])]),
        )
        .unwrap()
        .id
        .unwrap();
    pages
        .run(
            "setRedirect",
            &v(&[("id", &[&copy]), ("redirectTo", &["t"])]),
        )
        .unwrap();
    let r = pages.run("resolve", &v(&[("path", &["t2"])])).unwrap();
    assert_eq!(r.values["outcome"], ["resolved"]);
    assert_eq!(r.id.as_deref(), Some(id.as_str()));
    let exported = pages
        .run(
            "export",
            &v(&[("id", &[&id]), ("mediaType", &["text/markdown"])]),
        )
        .unwrap();
    assert!(exported.values["text"][0].contains('T'));
    pages.run("trash", &v(&[("id", &[&copy])])).unwrap();
    let trash = backend(&w, "TrashEntry");
    let entries = trash.list(&Values::new()).unwrap();
    assert_eq!(entries.len(), 1);
    trash.run("restore", &v(&[("id", &[&copy])])).unwrap();
    assert!(trash.list(&Values::new()).unwrap().is_empty());
    assert_eq!(pages.list(&Values::new()).unwrap().len(), 2);
}

#[test]
fn folders_revisions_and_links() {
    let w = wiki();
    let folders = backend(&w, "Folder");
    let space = folders
        .run("create", &v(&[("name", &["Manual"])]))
        .unwrap()
        .id
        .unwrap();
    let sub = folders
        .run("create", &v(&[("parent", &[&space]), ("name", &["Start"])]))
        .unwrap()
        .id
        .unwrap();
    folders
        .run("rename", &v(&[("id", &[&sub]), ("name", &["Begin"])]))
        .unwrap();
    let listed = folders.list(&Values::new()).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|r| r.values["name"] == ["Begin"]));
    assert_eq!(folders.list(&v(&[("name", &["begin"])])).unwrap().len(), 1);
    folders
        .run("reorder", &v(&[("id", &[&sub]), ("placement", &["first"])]))
        .unwrap();
    let page = create(&w, "manual/p", "see [x](manual/q)\n");
    let revs = backend(&w, "Revision");
    let list = revs.list(&v(&[("page", &[&page])])).unwrap();
    assert_eq!(list.len(), 1);
    let got = revs.run("get", &v(&[("id", &[&list[0].id])])).unwrap();
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
    folders.run("delete", &v(&[("id", &[&sub])])).unwrap();
    assert_eq!(folders.list(&Values::new()).unwrap().len(), 1);
}
