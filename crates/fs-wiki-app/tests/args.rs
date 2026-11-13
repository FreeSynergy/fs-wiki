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

// Unix paths (`/data` is not absolute on Windows, where the data folder is `%LOCALAPPDATA%`).
#[cfg(unix)]
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

#[test]
fn the_old_store_file_of_a_folder_is_found_and_only_when_it_is_there() {
    use fs_wiki_app::store_path::old_store_file;
    let dir = std::env::temp_dir().join(format!("fs-wiki-old-store-{}", std::process::id()));
    let (data, docs) = (dir.join("data"), dir.join("docs"));
    std::fs::create_dir_all(&docs).unwrap();
    assert_eq!(old_store_file(&data, &docs), None);
    let old = origins_file(&data, &docs).with_extension("db");
    std::fs::create_dir_all(old.parent().unwrap()).unwrap();
    std::fs::write(&old, b"x").unwrap();
    assert_eq!(old_store_file(&data, &docs), Some(old));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_facet_is_read_by_the_shared_parser_of_fs_interfaces() {
    assert_eq!(parse(&args(&[])).unwrap().facet, None);
    let some = parse(&args(&["--facet", "identity"])).unwrap();
    assert_eq!(some.facet.as_deref(), Some("identity"));
    assert!(parse(&args(&["--facet"])).is_err(), "needs a value");
    assert!(parse(&args(&["--facet", "Not A Name"])).is_err());
}

#[test]
fn sources_have_a_prefix_and_a_docs_root_names_the_four_of_the_documentation() {
    let a = parse(&args(&[
        "--read",
        "--source",
        "en=/d/en",
        "--source",
        "manual/de=/d/de",
        "--nav",
        "/d/mkdocs.yml",
    ]))
    .unwrap();
    assert!(a.read);
    assert_eq!(
        a.sources,
        [
            ("en".to_owned(), "/d/en".into()),
            ("manual/de".to_owned(), "/d/de".into())
        ]
    );
    assert_eq!(a.nav, Some("/d/mkdocs.yml".into()));
    for bad in [
        "en", "=/d", "../x=/d", "a//b=/d", "a#b=/d", "/a=/d", "a/=/d",
    ] {
        assert!(parse(&args(&["--source", bad])).is_err(), "{bad}");
    }
    let root = temp_dir("args-root");
    for d in ["docs/en", "docs/de", "conventions/decisions"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("mkdocs.yml"), "site_name: x\n").unwrap();
    let r = parse(&args(&["--docs-root", &root.display().to_string()])).unwrap();
    let prefixes: Vec<&str> = r.sources.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(prefixes, ["en", "de", "adr"], "only folders that exist");
    assert_eq!(r.nav, Some(root.join("mkdocs.yml")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_option_of_the_help_is_known_to_the_parser_and_the_dispatch_matches() {
    use fs_wiki_app::gui::{Mode, mode};
    // An option the help names but the parser does not know (or a window nobody can reach) is the bug that hid the faces
    // behind `--read`.
    let options: Vec<&str> = fs_wiki_app::args::HELP
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|w| w.starts_with("--"))
        .collect();
    assert!(options.len() >= 18, "{options:?}");
    for option in &options {
        let said = parse(&args(&[option])).err().unwrap_or_default();
        assert!(!said.contains("unknown option"), "{option}: {said}");
    }
    let mode_of = |a: &[&str]| mode(&parse(&args(a)).unwrap());
    assert_eq!(mode_of(&[]), Mode::Wiki, "the desktop file: the faces");
    assert_eq!(mode_of(&["--socket", "s", "--frontend", "b"]), Mode::Wiki);
    assert_eq!(mode_of(&["--read"]), Mode::Wiki);
    assert_eq!(mode_of(&["--reader"]), Mode::Reader);
    assert_eq!(mode_of(&["--admin"]), Mode::Admin);
    assert_eq!(mode_of(&["--check"]), Mode::Admin);
}
