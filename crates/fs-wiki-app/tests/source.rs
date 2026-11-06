//! The wiki as the reader's data source: reading, links, search, editing, export.
//!
//! Pattern: Integration tests of the Adapter over `Texts`.

mod common;

use std::sync::Arc;

use common::{fingerprint, temp_dir, wiki, write_files};
use fs_components_workbenches::{DataSource, LinkTarget, Reader};
use fs_wiki_app::{PROBLEMS_PATH, WikiSource, import_folder};

fn source(
    files: &[(&str, &str)],
    name: &str,
) -> (tokio::runtime::Runtime, std::path::PathBuf, WikiSource) {
    let (rt, wiki) = wiki();
    let dir = temp_dir(name);
    write_files(&dir, files);
    let report = rt.block_on(import_folder(wiki.as_ref(), &dir));
    let src = WikiSource::new(wiki, rt.handle().clone(), report.failures);
    (rt, dir, src)
}

#[test]
fn lists_reads_finds_and_follows_relative_links() {
    let (_rt, dir, src) = source(
        &[
            ("index.md", "# Home\n\nsee [b](a/b.md)\n"),
            (
                "a/b.md",
                "# Bee\n\nunique-needle here\n\n[up](../index.md)\n",
            ),
        ],
        "read",
    );
    let titles: Vec<String> = src
        .entries("")
        .unwrap()
        .into_iter()
        .map(|e| e.title)
        .collect();
    assert_eq!(titles, ["Bee", "Home"]);
    assert_eq!(src.start().as_deref(), Some("index.md"));
    assert_eq!(src.read("a/b.md").unwrap().title, "Bee");
    assert!(src.read("nope.md").unwrap_err().0.contains("no page"));
    let hits = src.find("unique-needle").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "a/b.md");
    assert_eq!(
        src.resolve("a/b.md", "../index.md"),
        LinkTarget::Page("index.md".into())
    );
    assert_eq!(
        src.resolve("index.md", "zzz.md"),
        LinkTarget::Missing("zzz.md".into())
    );
    let mut reader = Reader::new(Arc::new(src));
    reader.open("a/b.md");
    reader.follow("../index.md");
    assert_eq!(reader.current_path(), Some("index.md"));
    reader.back();
    assert_eq!(reader.current_path(), Some("a/b.md"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn failed_imports_are_a_page_in_the_list() {
    let (_rt, dir, src) = source(&[("ok.md", "# Ok\n"), ("bad#.md", "# bad\n")], "problems");
    let entries = src.entries("").unwrap();
    assert_eq!(entries[0].path, PROBLEMS_PATH);
    assert_eq!(entries[0].title, "Import problems (1)");
    let doc = src.read(PROBLEMS_PATH).unwrap();
    assert!(
        doc.blocks
            .iter()
            .any(|b| b.plain_text().contains("bad#.md"))
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn saving_changes_the_database_and_never_the_file() {
    let (_rt, dir, src) = source(&[("p.md", "# P\n\nold text\n")], "save");
    let before = fingerprint(&dir);
    let text = src.source_text("p.md").unwrap();
    assert!(text.contains("old text"));
    src.save("p.md", &text.replace("old", "new")).unwrap();
    assert!(src.source_text("p.md").unwrap().contains("new text"));
    assert!(
        src.read("p.md")
            .unwrap()
            .blocks
            .iter()
            .any(|b| b.plain_text() == "new text")
    );
    assert_eq!(
        fingerprint(&dir),
        before,
        "saving must not write the folder"
    );
    assert!(!src.can_export());
    assert!(src.export("p.md").is_err());
    assert_eq!(fingerprint(&dir), before);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn export_writes_only_when_asked_and_only_to_the_given_folder() {
    let (rt, dir, src) = source(&[("p.md", "# P\n\nbody\n")], "export");
    let out = temp_dir("export-out");
    let src = src.with_export_root(out.clone());
    src.save("p.md", "# P\n\nedited body\n").unwrap();
    assert!(!out.join("p.md").exists(), "saving is not exporting");
    assert!(src.can_export());
    let message = src.export("p.md").unwrap();
    assert!(message.contains("p.md"));
    assert!(
        std::fs::read_to_string(out.join("p.md"))
            .unwrap()
            .contains("edited body")
    );
    assert!(
        std::fs::read_to_string(dir.join("p.md"))
            .unwrap()
            .contains("body\n")
    );
    assert!(
        !std::fs::read_to_string(dir.join("p.md"))
            .unwrap()
            .contains("edited")
    );
    drop(rt);
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(out).unwrap();
}
