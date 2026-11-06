//! The command line and the default place of the store.
//!
//! Pattern: Example-based tests of a parser and a pure function.

mod common;

use common::temp_dir;
use fs_wiki_app::args::{StoreChoice, parse};
use fs_wiki_app::store_path::store_file;

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(ToString::to_string).collect()
}

#[test]
fn the_store_is_a_file_by_default_and_memory_only_when_asked() {
    assert_eq!(parse(&args(&[])).unwrap().store, StoreChoice::Default);
    assert_eq!(
        parse(&args(&["--store", "memory"])).unwrap().store,
        StoreChoice::Memory
    );
    assert_eq!(
        parse(&args(&["--store", "./x.db"])).unwrap().store,
        StoreChoice::File("./x.db".into())
    );
}

#[test]
fn every_docs_folder_has_its_own_store_in_the_data_directory() {
    let data = temp_dir("args-data");
    let (a, b) = (data.join("docs-a"), data.join("other/docs-a"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let (fa, fb) = (store_file(&data, &a), store_file(&data, &b));
    assert_ne!(fa, fb, "same name, other folder");
    assert_eq!(fa, store_file(&data, &a.join("../docs-a")), "same folder");
    assert!(fa.starts_with(data.join("freesynergy/fs-wiki")));
    assert!(fa.extension().is_some_and(|e| e == "db"));
    std::fs::remove_dir_all(data).unwrap();
}
