//! The records of imported files: they survive a restart, replace each other by file, and a damaged
//! file is told, not guessed at.
//!
//! Pattern: Integration tests of a Repository over a real file in a real folder.

mod common;

use std::os::unix::fs::PermissionsExt;

use common::temp_dir;
use fs_objects::ObjectId;
use fs_wiki_app::{FileOrigins, OriginsError};
use fs_wiki_core::{FileOrigin, Origins};

fn record(file: &str, hash: &str) -> FileOrigin {
    FileOrigin {
        file: file.into(),
        hash: hash.into(),
        text: ObjectId::new(),
        text_hash: format!("text-{hash}"),
        deleted: false,
    }
}

#[tokio::test]
async fn records_survive_a_restart_and_the_last_one_of_a_file_wins() {
    let dir = temp_dir("or-restart");
    let file = dir.join("deep/er/docs.origins");
    let first = FileOrigins::open(&file).unwrap();
    let (a1, b, a2) = (
        record("a.md", "1"),
        record("b/c.md", "1"),
        record("a.md", "2"),
    );
    for r in [&a1, &b, &a2] {
        first.set_origin(r).await.unwrap();
    }
    assert_eq!(first.origins().await.unwrap(), [a2.clone(), b.clone()]);
    drop(first);
    let again = FileOrigins::open(&file).unwrap();
    assert_eq!(again.origins().await.unwrap(), [a2, b]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn the_file_holds_each_record_once_after_a_start_and_is_private() {
    let dir = temp_dir("or-compact");
    let file = dir.join("docs.origins");
    let origins = FileOrigins::open(&file).unwrap();
    for i in 0..20 {
        origins
            .set_origin(&record("same.md", &i.to_string()))
            .await
            .unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&file).unwrap().lines().count(),
        20,
        "a log"
    );
    drop(origins);
    let origins = FileOrigins::open(&file).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 1);
    assert_eq!(origins.origins().await.unwrap()[0].hash, "19");
    let mode = std::fs::metadata(&file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert!(
        !file.with_extension("tmp").exists(),
        "no temporary file left"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_torn_last_line_is_ignored_and_a_bad_line_elsewhere_is_an_error() {
    let dir = temp_dir("or-torn");
    let file = dir.join("docs.origins");
    let origins = FileOrigins::open(&file).unwrap();
    origins.set_origin(&record("a.md", "1")).await.unwrap();
    drop(origins);
    let good = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, format!("{good}{{\"file\":\"b.md\",\"ha")).unwrap();
    let origins = FileOrigins::open(&file).unwrap();
    assert_eq!(
        origins.origins().await.unwrap().len(),
        1,
        "the torn line is dropped"
    );
    drop(origins);
    let damaged = format!("not json\n{good}");
    std::fs::write(&file, &damaged).unwrap();
    let e = FileOrigins::open(&file).err().expect("an error");
    assert!(matches!(e, OriginsError::Damaged { line: 1, .. }), "{e}");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        damaged,
        "left as it was"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_whole_documentation_of_records_is_written_in_a_moment() {
    let dir = temp_dir("or-many");
    let origins = FileOrigins::open(&dir.join("docs.origins")).unwrap();
    let started = std::time::Instant::now();
    for i in 0..1400 {
        origins
            .set_origin(&record(&format!("dir/{i}.md"), "h"))
            .await
            .unwrap();
    }
    assert_eq!(origins.origins().await.unwrap().len(), 1400);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_second_program_on_the_same_records_is_refused_until_the_first_lets_go() {
    let dir = temp_dir("or-lock");
    let file = dir.join("docs.origins");
    let first = FileOrigins::open(&file).unwrap();
    let e = FileOrigins::open(&file).err().expect("refused");
    assert!(matches!(e, OriginsError::Busy { .. }), "{e}");
    first.set_origin(&record("a.md", "1")).await.unwrap();
    drop(first);
    let again = FileOrigins::open(&file).unwrap();
    assert_eq!(again.origins().await.unwrap().len(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn records_files_that_differ_only_in_their_extension_do_not_share_helper_files() {
    let dir = temp_dir("or-names");
    let (a, b) = (dir.join("a.origins"), dir.join("a.json"));
    let (first, second) = (
        FileOrigins::open(&a).unwrap(),
        FileOrigins::open(&b).unwrap(),
    );
    first.set_origin(&record("one.md", "1")).await.unwrap();
    second.set_origin(&record("two.md", "1")).await.unwrap();
    assert_eq!(first.origins().await.unwrap()[0].file, "one.md");
    assert_eq!(second.origins().await.unwrap()[0].file, "two.md");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["a.json", "a.json.lock", "a.origins", "a.origins.lock"]
    );
    std::fs::remove_dir_all(dir).unwrap();
}
