//! Export under attack and failure: links swapped in, hard links, half written files, diffs.
//!
//! Pattern: Example-based tests of a Guard on a real temp folder.

mod common;

use common::{temp_dir, write_files};
use fs_wiki_app::export::{ExportError, export_markdown_checked};
use fs_wiki_app::export_markdown_to;

#[cfg(unix)]
#[test]
fn a_linked_folder_with_the_same_content_is_not_reported_as_exported() {
    let target = temp_dir("exs-same");
    let outside = temp_dir("exs-same-outside");
    write_files(&outside, &[("same.md", "same\n")]);
    std::os::unix::fs::symlink(&outside, target.join("dirlink")).unwrap();
    let r = export_markdown_to(
        "dirlink/same.md",
        &target.display().to_string(),
        "same\n",
        false,
    );
    let message = r.expect_err("must refuse").message;
    assert!(message.contains("symbolic link"), "{message}");
    std::fs::remove_dir_all(target).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn a_hard_link_is_not_written_through() {
    let target = temp_dir("exs-hard");
    let outside = temp_dir("exs-hard-outside");
    write_files(&outside, &[("secret.md", "outside\n")]);
    std::fs::hard_link(outside.join("secret.md"), target.join("p.md")).unwrap();
    export_markdown_to("p.md", &target.display().to_string(), "new\n", true).unwrap();
    assert_eq!(
        std::fs::read_to_string(outside.join("secret.md")).unwrap(),
        "outside\n"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("p.md")).unwrap(),
        "new\n"
    );
    std::fs::remove_dir_all(target).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

#[test]
fn no_half_written_file_or_leftover_temp_file() {
    let target = temp_dir("exs-atomic");
    write_files(&target, &[("p.md", "old\n")]);
    export_markdown_to("p.md", &target.display().to_string(), "new\n", true).unwrap();
    export_markdown_to("a/b.md", &target.display().to_string(), "b\n", false).unwrap();
    let mut names: Vec<_> = std::fs::read_dir(&target)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["a", "p.md"]);
    let inner: Vec<_> = std::fs::read_dir(target.join("a")).unwrap().collect();
    assert_eq!(inner.len(), 1);
    std::fs::remove_dir_all(target).unwrap();
}

#[test]
fn a_conflict_carries_both_texts_so_a_screen_can_show_a_diff() {
    let target = temp_dir("exs-diff");
    write_files(&target, &[("p.md", "line1\nold\n")]);
    let err = export_markdown_checked("p.md", &target.display().to_string(), "line1\nnew\n", false)
        .unwrap_err();
    let ExportError::Differs(d) = err else {
        panic!("expected a difference");
    };
    assert_eq!(
        (d.on_disk.as_str(), d.wanted.as_str()),
        ("line1\nold\n", "line1\nnew\n")
    );
    assert!(d.file.ends_with("p.md"));
    std::fs::remove_dir_all(target).unwrap();
}
