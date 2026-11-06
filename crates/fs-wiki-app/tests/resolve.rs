//! Following links of Markdown pages.
//!
//! Pattern: Table-driven tests of a pure function.

use fs_components_workbenches::LinkTarget::{External, Missing, Page};
use fs_wiki_app::resolve;

fn pages(p: &str) -> bool {
    [
        "index.md",
        "a/b.md",
        "a/c/index.md",
        "x y.md",
        "a/README.md",
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
