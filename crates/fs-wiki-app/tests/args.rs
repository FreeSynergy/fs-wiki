//! The command line and the default place of the records.
//!
//! Pattern: Example-based tests of a parser and a pure function.

mod common;

use common::temp_dir;
use fs_wiki_app::args::parse;
use fs_wiki_app::store_path::origins_file;

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(ToString::to_string).collect()
}

#[test]
fn the_server_is_found_by_socket_and_the_records_by_origins_and_nothing_is_a_store() {
    let none = parse(&args(&[])).unwrap();
    assert_eq!((none.socket, none.origins), (None, None));
    let some = parse(&args(&["--socket", "/run/w.sock", "--origins", "./o"])).unwrap();
    assert_eq!(some.socket, Some("/run/w.sock".into()));
    assert_eq!(some.origins, Some("./o".into()));
    assert!(parse(&args(&["--socket"])).is_err(), "needs a value");
    let old = parse(&args(&["--store", "memory"])).unwrap_err();
    assert!(old.contains("server"), "{old}");
}

#[test]
fn every_docs_folder_has_its_own_store_in_the_data_directory() {
    let data = temp_dir("args-data");
    let (a, b) = (data.join("docs-a"), data.join("other/docs-a"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let (fa, fb) = (origins_file(&data, &a), origins_file(&data, &b));
    assert_ne!(fa, fb, "same name, other folder");
    assert_eq!(fa, origins_file(&data, &a.join("../docs-a")), "same folder");
    assert!(fa.starts_with(data.join("freesynergy/fs-wiki")));
    assert!(fa.extension().is_some_and(|e| e == "origins"));
    std::fs::remove_dir_all(data).unwrap();
}

#[test]
fn an_empty_or_relative_data_directory_is_treated_as_unset() {
    use fs_wiki_app::store_path::data_dir_from;
    use std::ffi::OsString;
    let os = |s: &str| Some(OsString::from(s));
    assert_eq!(
        data_dir_from(os("/data"), os("/home/u")),
        Some("/data".into())
    );
    for bad in ["", "relative/dir"] {
        assert_eq!(
            data_dir_from(os(bad), os("/home/u")),
            Some("/home/u/.local/share".into()),
            "{bad:?}"
        );
    }
    assert_eq!(data_dir_from(os(""), os("")), None);
    assert_eq!(data_dir_from(None, os("rel")), None);
}
