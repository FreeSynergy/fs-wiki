//! The wiki as the reader's data source: reading, links, search, editing, export.
//!
//! Pattern: Integration tests of the Adapter over `Texts`.

mod common;

use std::sync::Arc;

use common::{fingerprint, temp_dir, wiki, write_files};
use fs_components_workbenches::{DocumentSource, ErrorKind, Labels, LinkTarget, Reader};
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
    assert_eq!(src.read("nope.md").unwrap_err().kind, ErrorKind::NotFound);
    let hits = src.find("unique-needle").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "a/b.md");
    assert_eq!(
        src.resolve("a/b.md", "../index.md"),
        LinkTarget::Document("index.md".into())
    );
    assert_eq!(
        src.resolve("index.md", "zzz.md"),
        LinkTarget::Missing("zzz.md".into())
    );
    let mut reader = Reader::new(Arc::new(src), Labels::default());
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
    let editable = src.source_text("p.md").unwrap();
    assert!(editable.text.contains("old text"));
    src.save(
        "p.md",
        &editable.text.replace("old", "new"),
        editable.revision.as_deref(),
    )
    .unwrap();
    assert!(src.source_text("p.md").unwrap().text.contains("new text"));
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
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_save_from_an_old_revision_is_a_conflict_and_writes_nothing() {
    let (rt, dir, src) = source(&[("p.md", "# P\n\nold text\n")], "conflict");
    let mine = src.source_text("p.md").unwrap();
    assert_eq!(src.read("p.md").unwrap().revision, mine.revision);
    // Someone else saves in between.
    let other = src.source_text("p.md").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    src.save(
        "p.md",
        &other.text.replace("old", "THEIRS"),
        other.revision.as_deref(),
    )
    .unwrap();
    let err = src
        .save(
            "p.md",
            &mine.text.replace("old", "mine"),
            mine.revision.as_deref(),
        )
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict);
    assert!(src.source_text("p.md").unwrap().text.contains("THEIRS"));
    drop(rt);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_reader_keeps_the_draft_on_a_conflict() {
    let (_rt, dir, src) = source(&[("p.md", "# P\n\ntext\n")], "reader-conflict");
    let src = Arc::new(src);
    let mut reader = Reader::new(src.clone(), Labels::default());
    reader.open("p.md");
    reader.begin_edit();
    let theirs = src.source_text("p.md").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    src.save(
        "p.md",
        &theirs.text.replace("text", "theirs"),
        theirs.revision.as_deref(),
    )
    .unwrap();
    reader.set_draft("# P\n\nmine\n");
    reader.save();
    assert!(
        reader
            .editing()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("Someone else")
    );
    assert!(src.source_text("p.md").unwrap().text.contains("theirs"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_problems_page_is_not_editable() {
    let (_rt, dir, src) = source(&[("bad#.md", "x")], "noedit");
    assert!(!src.can_edit(PROBLEMS_PATH));
    assert!(src.can_edit("anything.md"));
    std::fs::remove_dir_all(dir).unwrap();
}
