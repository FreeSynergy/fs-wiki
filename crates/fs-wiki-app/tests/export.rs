//! Export: the only code that writes a file. Tests of what it refuses and what it never does.
//!
//! Pattern: Example-based tests of a Guard.

mod common;

use common::{fingerprint, temp_dir, write_files};
use fs_components_workbenches::ErrorKind;
use fs_wiki_app::export_markdown_to;

fn target(name: &str) -> (std::path::PathBuf, String) {
    let dir = temp_dir(name);
    let t = dir.display().to_string();
    (dir, t)
}

#[test]
fn writes_below_the_target_and_creates_folders() {
    let (dir, t) = target("ex-ok");
    let msg = export_markdown_to("a/b.md", &t, "# B\n", false).unwrap();
    assert!(msg.contains("b.md"));
    assert_eq!(
        std::fs::read_to_string(dir.join("a/b.md")).unwrap(),
        "# B\n"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn refuses_paths_that_leave_the_target() {
    let (dir, t) = target("ex-escape");
    let outside = dir
        .parent()
        .unwrap()
        .join(format!("fs-escape-{}", std::process::id()));
    for path in [
        "../x.md",
        "a/../../x.md",
        "/abs.md",
        "a\\b.md",
        "",
        "a//b.md",
        "./x.md",
    ] {
        let r = export_markdown_to(path, &t, "x", true);
        assert!(r.is_err(), "{path:?} must be refused");
    }
    assert!(!outside.exists());
    assert!(!dir.parent().unwrap().join("x.md").exists());
    assert_eq!(fingerprint(&dir), vec![]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn never_replaces_silently() {
    let (dir, t) = target("ex-overwrite");
    write_files(&dir, &[("p.md", "old")]);
    let err = export_markdown_to("p.md", &t, "new", false).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict);
    assert_eq!(std::fs::read_to_string(dir.join("p.md")).unwrap(), "old");
    export_markdown_to("p.md", &t, "new", true).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("p.md")).unwrap(), "new");
    // The same content is no conflict.
    export_markdown_to("p.md", &t, "new", false).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn does_not_follow_symbolic_links_out() {
    let (dir, t) = target("ex-link");
    let outside = temp_dir("ex-link-outside");
    std::os::unix::fs::symlink(&outside, dir.join("link")).unwrap();
    assert!(export_markdown_to("link/x.md", &t, "x", true).is_err());
    assert!(!outside.join("x.md").exists());
    std::os::unix::fs::symlink(outside.join("target.md"), dir.join("file.md")).unwrap();
    assert!(export_markdown_to("file.md", &t, "x", true).is_err());
    assert!(!outside.join("target.md").exists());
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}
