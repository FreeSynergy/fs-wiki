//! Tests of the name check, the index page and the title rewrite.
//!
//! Pattern: none (tests).

use super::*;

#[test]
fn a_name_is_checked_before_anything_is_asked() {
    assert_eq!(check_name("  Hand   book ").as_deref(), Ok("Hand book"));
    assert_eq!(check_name("   "), Err(CreateError::EmptyTitle));
    assert_eq!(check_name("a/b"), Err(CreateError::BadChars));
    assert_eq!(check_name("a\\b"), Err(CreateError::BadChars));
    assert_eq!(check_name("a\u{7}b"), Err(CreateError::BadChars));
    assert_eq!(check_name("?!"), Err(CreateError::NoLetter));
    assert_eq!(check_name(" Index "), Err(CreateError::ReservedName));
    assert!(check_name("Main index").is_ok() && check_name("Reindex").is_ok());
}

#[test]
fn only_a_file_named_index_is_an_index_page() {
    assert!(is_index("index.md") && is_index("a/index.md"));
    assert!(!is_index("reindex.md") && !is_index("main-index.md") && !is_index("a/reindex.md"));
}

#[test]
fn a_page_stays_in_its_folder_when_renamed() {
    assert_eq!(sibling("a.md", "New Name").as_deref(), Some("new-name.md"));
    assert_eq!(sibling("x/y/a.md", "B").as_deref(), Some("x/y/b.md"));
}

#[test]
fn the_title_of_the_text_follows_the_name() {
    assert_eq!(retitle("# Old\n\ntext\n", "Old", "New"), "# New\n\ntext\n");
    assert_eq!(retitle("text\n", "Old", "New"), "# New\n\ntext\n");
    assert_eq!(retitle("\n# Old\nx\n", "Old", "New"), "\n# New\nx\n");
    // With a title in the front matter the heading changes only when it said the old title.
    assert_eq!(
        retitle("---\ntitle: Old\n---\n# Old\nx\n", "Old", "N \"q\""),
        "---\ntitle: \"N \\\"q\\\"\"\n---\n# N \"q\"\nx\n"
    );
    assert_eq!(
        retitle("---\ntitle: Old\n---\n# Else\nx\n", "Old", "New"),
        "---\ntitle: \"New\"\n---\n# Else\nx\n"
    );
}
