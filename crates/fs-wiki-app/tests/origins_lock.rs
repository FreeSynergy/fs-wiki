//! The helper files and the lock of the records: names that belong to someone else are left alone,
//! and the lock is held until the last write has ended.
//!
//! Pattern: Integration tests of a Repository over real files in a real folder.

mod common;

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
async fn a_file_named_like_the_stem_of_the_records_is_left_alone() {
    // The temporary file of `a.origins` must not be `a.tmp`: that name belongs to someone else.
    let dir = temp_dir("or-stem");
    std::fs::write(dir.join("a.tmp"), "mine").unwrap();
    let origins = FileOrigins::open(&dir.join("a.origins")).unwrap();
    origins.set_origin(&record("one.md", "1")).await.unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("a.tmp")).unwrap(), "mine");
    drop(origins);
    let again = FileOrigins::open(&dir.join("a.origins")).unwrap();
    assert_eq!(again.origins().await.unwrap().len(), 1);
    assert_eq!(std::fs::read_to_string(dir.join("a.tmp")).unwrap(), "mine");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn the_lock_is_held_until_the_last_write_has_ended_even_after_a_drop() {
    use futures::FutureExt;
    let dir = temp_dir("or-drop");
    let file = dir.join("docs.origins");
    let origins = FileOrigins::open(&file).unwrap();
    let records: Vec<_> = (0..200).map(|i| record(&format!("{i}.md"), "1")).collect();
    // Start the writes (polling once hands each to a thread for blocking work) and let go at once.
    for r in &records {
        let _ = origins.set_origin(r).now_or_never();
    }
    drop(origins);
    let e = FileOrigins::open(&file)
        .err()
        .expect("writes are still under way");
    assert!(matches!(e, OriginsError::Busy { .. }), "{e}");
    // When they have ended the lock is free, and every record was written.
    let started = std::time::Instant::now();
    let reopened = loop {
        match FileOrigins::open(&file) {
            Ok(o) => break o,
            Err(_) if started.elapsed() < std::time::Duration::from_secs(30) => {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            Err(e) => panic!("{e}"),
        }
    };
    assert_eq!(reopened.origins().await.unwrap().len(), 200);
    std::fs::remove_dir_all(dir).unwrap();
}
