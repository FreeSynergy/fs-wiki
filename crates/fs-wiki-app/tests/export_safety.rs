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

#[cfg(unix)]
#[test]
fn replacing_a_file_keeps_its_mode_and_a_new_file_is_never_executable() {
    use std::os::unix::fs::PermissionsExt;
    let target = temp_dir("exs-mode");
    write_files(&target, &[("private.md", "old\n"), ("script.md", "old\n")]);
    let set = |name: &str, mode: u32| {
        std::fs::set_permissions(target.join(name), std::fs::Permissions::from_mode(mode)).unwrap();
    };
    set("private.md", 0o600);
    set("script.md", 0o755);
    let t = target.display().to_string();
    export_markdown_to("private.md", &t, "new\n", true).unwrap();
    export_markdown_to("script.md", &t, "new\n", true).unwrap();
    export_markdown_to("fresh.md", &t, "new\n", true).unwrap();
    let mode = |name: &str| {
        std::fs::metadata(target.join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(
        mode("private.md"),
        0o600,
        "a private file must stay private"
    );
    assert_eq!(mode("script.md"), 0o755);
    assert_eq!(mode("fresh.md") & 0o111, 0);
    std::fs::remove_dir_all(target).unwrap();
}

#[test]
fn a_long_file_name_and_a_leftover_temporary_file_do_not_stop_the_export() {
    let target = temp_dir("exs-long");
    let t = target.display().to_string();
    let long = format!("{}.md", "n".repeat(252));
    export_markdown_to(&long, &t, "x\n", false).unwrap();
    export_markdown_to(&long, &t, "y\n", true).unwrap();
    // What an earlier crashed export may have left, whatever its name.
    let pid = std::process::id();
    write_files(
        &target,
        &[
            (&format!(".p.md.tmp-{pid}"), "junk"),
            (&format!(".fs-wiki-export-{pid}-0"), "junk"),
            (&format!(".fs-wiki-export-{pid}-1"), "junk"),
        ],
    );
    export_markdown_to("p.md", &t, "z\n", false).unwrap();
    assert_eq!(std::fs::read_to_string(target.join("p.md")).unwrap(), "z\n");
    std::fs::remove_dir_all(target).unwrap();
}
