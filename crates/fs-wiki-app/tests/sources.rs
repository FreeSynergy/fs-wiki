//! Several folders in one wiki: a prefix per source, the order of the documentation's navigation, attachments named, and the
//! round trip Markdown to wiki and back, byte for byte (also against the real documentation, `--ignored`).
//!
//! Pattern: Integration tests over a real in-memory wiki and real temp folders.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{temp_dir, wiki, write_files};
use fs_components_workbenches::DocumentSource;
use fs_wiki_app::WikiSource;
use fs_wiki_app::import::import_sources;
use fs_wiki_app::nav::Nav;
use fs_wiki_core::{MARKDOWN, Texts};

const LIMIT: Duration = Duration::from_secs(120);

fn markdown_files(dir: &Path, below: &Path, out: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() && !e.file_name().to_string_lossy().starts_with('.') {
            markdown_files(&p, below, out);
        } else if p.extension().is_some_and(|x| x == "md") && p.is_file() {
            let rel = p.strip_prefix(below).unwrap();
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn sources_go_below_their_prefix_in_one_wiki_in_the_order_of_the_navigation() {
    let (rt, wiki) = wiki();
    let (en, de) = (temp_dir("src-en"), temp_dir("src-de"));
    for root in [&en, &de] {
        write_files(
            root,
            &[
                ("zeta.md", "# Zeta\n"),
                ("index.md", "# Home\n"),
                ("manifesto.md", "# Why\n"),
                ("programs/index.md", "# Programs\n"),
                ("programs/.pages", "title: 1 Programs\n"),
                ("start/b2.md", "# B2\n"),
                ("start/b10.md", "# B10\n"),
                ("start/a.md", "# A\n"),
                ("start/.pages", "title: 0 Start\n"),
                ("assets/logo.svg", "<svg/>"),
            ],
        );
    }
    let conf = temp_dir("src-conf");
    let mkdocs = conf.join("mkdocs.yml");
    std::fs::write(
        &mkdocs,
        "plugins:\n  - i18n:\n      languages:\n        - locale: en\n          nav_translations:\n            6 Manifesto: Manifesto\n",
    )
    .unwrap();
    let sources = vec![("en".to_owned(), en.clone()), ("de".to_owned(), de.clone())];
    let report = rt.block_on(import_sources(&**wiki, &sources, LIMIT));
    assert_eq!(report.imported, 14, "{report:?}");
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    // The attachments are named, with their prefix, their size and their hash: never dropped silently.
    let named: Vec<&str> = report.attachments.iter().map(|a| a.path.as_str()).collect();
    assert_eq!(named, ["en/assets/logo.svg", "de/assets/logo.svg"]);
    assert!(
        report.attachments[0].reason.contains("6 bytes")
            && report.attachments[0].reason.contains("sha256"),
        "{:?}",
        report.attachments[0]
    );
    // Importing again changes nothing, and the second source's records do not make the first one's files "gone".
    let again = rt.block_on(import_sources(&**wiki, &sources, LIMIT));
    assert_eq!(
        (again.imported, again.unchanged, again.gone.len()),
        (0, 14, 0)
    );
    let (nav, notes) = Nav::read(&sources, Some(&mkdocs));
    assert!(notes.is_empty(), "{notes:?}");
    let source = WikiSource::from_report(wiki.texts(), rt.handle().clone(), again).with_nav(nav);
    let paths: Vec<String> = source
        .entries("en/")
        .unwrap()
        .into_iter()
        .map(|e| e.path)
        .collect();
    // index first; then by number (0 Start, 1 Programs, 6 Manifesto), within a folder the index, then natural order; the rest after.
    assert_eq!(
        paths,
        [
            "en/index.md",
            "en/start/a.md",
            "en/start/b2.md",
            "en/start/b10.md",
            "en/programs/index.md",
            "en/manifesto.md",
            "en/zeta.md"
        ]
    );
    assert_eq!(source.start().as_deref(), Some("en/index.md"));
    for d in [en, de, conf] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_page_comes_back_as_the_bytes_it_was_imported_from() {
    let (rt, wiki) = wiki();
    let dir = temp_dir("src-bytes");
    // Awkward on purpose: list styles, a rule, front matter, an admonition, a table, trailing spaces, no final newline.
    write_files(
        &dir,
        &[
            (
                "a.md",
                "---\ntitle: A\nhide:\n  - toc\n---\n\n# A\n\n* one\n* two\n\n\n- dash\n+ plus\n\n***emphasis chain***  \nline\n",
            ),
            (
                "b.md",
                "!!! note \"Title\"\n    body\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n---\n\n[x](a.md)",
            ),
        ],
    );
    let sources = vec![("p".to_owned(), dir.clone())];
    assert_eq!(
        rt.block_on(import_sources(&**wiki, &sources, LIMIT))
            .imported,
        2
    );
    for name in ["a.md", "b.md"] {
        let want = std::fs::read(dir.join(name)).unwrap();
        let entry = rt
            .block_on(wiki.get_by_path(&format!("p/{name}")))
            .unwrap()
            .unwrap();
        let got = rt.block_on(wiki.export_text(entry.id, MARKDOWN)).unwrap();
        assert_eq!(got.as_bytes(), want.as_slice(), "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// The sources of a checkout of the documentation, as `--docs-root` names them.
fn docs_sources(root: &Path) -> Vec<(String, PathBuf)> {
    [
        ("en", "docs/en"),
        ("de", "docs/de"),
        ("adr", "conventions/decisions"),
        ("research", "temp/research"),
    ]
    .into_iter()
    .map(|(p, d)| (p.to_owned(), root.join(d)))
    .filter(|(_, d)| d.is_dir())
    .collect()
}

#[test]
#[ignore = "reads the real documentation; FS_DOCS_ROOT=<checkout> cargo test --release --test sources -- --ignored --nocapture"]
fn the_real_documentation_comes_back_byte_for_byte() {
    let Ok(root) = std::env::var("FS_DOCS_ROOT") else {
        println!("FS_DOCS_ROOT not set; skipped");
        return;
    };
    let (rt, wiki) = wiki();
    let sources = docs_sources(Path::new(&root));
    let report = rt.block_on(import_sources(&**wiki, &sources, LIMIT));
    let (mut files, mut same, mut different, mut absent) = (0, 0, Vec::new(), Vec::new());
    for (prefix, dir) in &sources {
        let mut list = Vec::new();
        markdown_files(dir, dir, &mut list);
        list.sort();
        for rel in list {
            files += 1;
            let path = format!("{prefix}/{rel}");
            let Some(entry) = rt.block_on(wiki.get_by_path(&path)).unwrap() else {
                absent.push(path);
                continue;
            };
            let want = std::fs::read(dir.join(&rel)).unwrap();
            let got = rt.block_on(wiki.export_text(entry.id, MARKDOWN)).unwrap();
            if got.as_bytes() == want.as_slice() {
                same += 1;
            } else {
                different.push(path);
            }
        }
    }
    println!(
        "sources: {}, files: {files}, imported: {}, byte-identical: {same}, different: {}, not in the wiki: {}, failures: {}, attachments: {}, skipped: {}",
        sources.len(),
        report.imported,
        different.len(),
        absent.len(),
        report.failures.len(),
        report.attachments.len(),
        report.skipped.len()
    );
    for a in report.attachments.iter().take(20) {
        println!("attachment {}: {}", a.path, a.reason);
    }
    for f in report.failures.iter().chain(&report.skipped).take(20) {
        println!("not imported {}: {}", f.path, f.reason);
    }
    for d in different.iter().take(20) {
        println!("different {d}");
    }
    for a in absent.iter().take(20) {
        println!("absent {a}");
    }
    assert!(different.is_empty() && absent.is_empty() && report.failures.is_empty());
}
