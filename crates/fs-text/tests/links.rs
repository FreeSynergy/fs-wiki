//! Link table and rewriting.

use fs_text::*;
mod common;

#[test]
fn link_table_and_rewrite() {
    let mut a = import_markdown("See [[wiki/old-page]] and [[wiki/old-page#Goals]] and [x](wiki/old-page/sub) and [ext](https://e.org/wiki/old-page).\n\nShown [[wiki/old-page|label]].\n");
    let rows = extract_links(&a);
    assert_eq!(rows.len(), 5);
    assert!(rows[0].internal && !rows[3].internal);
    assert_eq!(rows[0].title, "wiki/old-page");
    let mut docs = vec![a.clone()];
    let changes = rewrite_links(&mut docs, "wiki/old-page", "archive/new");
    assert_eq!(changes.len(), 1);
    let out = export_markdown(&docs[0], ExportOptions::default());
    assert_eq!(out, "See [[archive/new]] and [[archive/new#Goals]] and [x](archive/new/sub) and [ext](https://e.org/wiki/old-page).\n\nShown [[archive/new|label]].\n");
    // undo
    for op in &changes[0].inverse {
        docs[0].apply(op).unwrap();
    }
    assert!(docs[0].same_content(&a));
    a.blocks.clear();
    assert_eq!(moved_path_check(), Some("b/c#h".to_string()));
}
fn moved_path_check() -> Option<String> {
    fs_text::links::moved_path("a/c#h", "a", "b")
}
