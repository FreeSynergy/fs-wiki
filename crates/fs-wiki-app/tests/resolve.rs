//! Following links of Markdown pages.
//!
//! Pattern: Table-driven tests of a pure function.

use fs_components_workbenches::LinkTarget::{Document as Page, External, Missing};
use fs_wiki_app::{resolve, resolve_with_folders};

fn pages(p: &str) -> bool {
    [
        "index.md",
        "a/b.md",
        "a/c/index.md",
        "x y.md",
        "a/README.md",
        "a:b.md",
    ]
    .contains(&p)
}

#[test]
fn relative_md_links_are_resolved_against_the_page() {
    assert_eq!(
        resolve("a/b.md", "../index.md", &pages),
        Page("index.md".into())
    );
    assert_eq!(resolve("index.md", "a/b.md", &pages), Page("a/b.md".into()));
    assert_eq!(
        resolve("a/b.md", "./c/index.md#top", &pages),
        Page("a/c/index.md".into())
    );
    assert_eq!(
        resolve("a/b.md", "/index.md", &pages),
        Page("index.md".into())
    );
}

#[test]
fn folders_and_missing_extensions_find_their_page() {
    assert_eq!(
        resolve("index.md", "a/c/", &pages),
        Page("a/c/index.md".into())
    );
    assert_eq!(
        resolve("index.md", "a/c", &pages),
        Page("a/c/index.md".into())
    );
    assert_eq!(resolve("index.md", "a/b", &pages), Page("a/b.md".into()));
    assert_eq!(
        resolve("index.md", "a/", &pages),
        Page("a/README.md".into())
    );
}

#[test]
fn anchors_encoding_and_same_page() {
    assert_eq!(resolve("a/b.md", "#section", &pages), Page("a/b.md".into()));
    assert_eq!(
        resolve("index.md", "x%20y.md", &pages),
        Page("x y.md".into())
    );
    assert_eq!(
        resolve("index.md", "a/b.md?x=1", &pages),
        Page("a/b.md".into())
    );
}

#[test]
fn outside_and_missing() {
    assert_eq!(
        resolve("index.md", "https://example.org/x", &pages),
        External("https://example.org/x".into())
    );
    assert_eq!(
        resolve("index.md", "mailto:a@b.c", &pages),
        External("mailto:a@b.c".into())
    );
    assert_eq!(
        resolve("index.md", "nope.md", &pages),
        Missing("nope.md".into())
    );
    assert_eq!(
        resolve("index.md", "../../x.md", &pages),
        Missing("../../x.md".into())
    );
}

#[test]
fn only_http_https_and_mailto_are_opened_outside() {
    for href in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "tel:+491234",
        "ssh://host/x",
        "//host/x",
        "FILE:///etc/passwd",
    ] {
        assert!(
            matches!(resolve("index.md", href, &pages), Missing(_)),
            "{href} must be shown, not opened"
        );
    }
    for href in [
        "HTTPS://example.org/x",
        "http://example.org",
        "MAILTO:a@b.c",
    ] {
        assert_eq!(
            resolve("index.md", href, &pages),
            External(href.to_string())
        );
    }
}

#[test]
fn a_colon_after_the_path_does_not_make_a_scheme() {
    assert_eq!(
        resolve("index.md", "a/b.md?u=http://h", &pages),
        Page("a/b.md".into())
    );
    assert_eq!(
        resolve("a/b.md", "../index.md#://", &pages),
        Page("index.md".into())
    );
    assert_eq!(
        resolve("a/b.md", "../x.md#://", &pages),
        Missing("x.md".into())
    );
}

#[test]
fn the_docs_root_and_folders_without_an_index_open_something() {
    assert_eq!(resolve("a/b.md", "../", &pages), Page("index.md".into()));
    assert_eq!(resolve("a/b.md", "..", &pages), Page("index.md".into()));
    let first = |folder: &str| (folder == "a/loose").then(|| "a/loose/one.md".to_string());
    assert_eq!(
        resolve_with_folders("index.md", "a/loose/", &pages, &first),
        Page("a/loose/one.md".into())
    );
    assert_eq!(
        resolve_with_folders("index.md", "a/empty/", &pages, &first),
        Missing("a/empty/".into())
    );
}

#[test]
fn a_file_name_with_a_colon_is_a_page_not_a_scheme() {
    assert_eq!(resolve("index.md", "a:b.md", &pages), Page("a:b.md".into()));
    assert!(matches!(resolve("index.md", "x:y.md", &pages), Missing(_)));
    assert!(matches!(
        resolve("index.md", "file://a:b.md", &pages),
        Missing(_)
    ));
}
