//! The real documentation as pages with language variants: how many pages, in how many languages, and the round trip of
//! every variant, byte for byte, back to its own file (two files for a page in two languages).
//!
//! Pattern: Integration test over the real documentation, read only (`--ignored`, `FS_DOCS_ROOT=<checkout>`).

mod common;

use std::path::Path;
use std::time::Duration;

use common::wiki;
use fs_wiki_app::import::import_sources;
use fs_wiki_app::variants::{PageIndex, VariantSource};
use fs_wiki_core::{MARKDOWN, Texts};

#[test]
#[ignore = "reads the real documentation; FS_DOCS_ROOT=<checkout> cargo test --release --test variants_real -- --ignored --nocapture"]
fn the_real_documentation_is_pages_with_variants_and_comes_back_as_its_files() {
    let Ok(root) = std::env::var("FS_DOCS_ROOT") else {
        println!("FS_DOCS_ROOT not set; skipped");
        return;
    };
    let root = Path::new(&root);
    // prefix, folder, family, language (as `--docs-root` names them)
    let config = [
        ("en", "docs/en", "docs", "en"),
        ("de", "docs/de", "docs", "de"),
        ("adr", "conventions/decisions", "adr", "de"),
        ("research", "temp/research", "research", "en"),
    ];
    let sources: Vec<(String, std::path::PathBuf)> = config
        .iter()
        .map(|(p, d, _, _)| ((*p).to_owned(), root.join(d)))
        .collect();
    let variants: Vec<VariantSource> = config
        .iter()
        .map(|(p, _, g, l)| VariantSource {
            prefix: (*p).into(),
            group: (*g).into(),
            lang: (*l).into(),
        })
        .collect();
    let (rt, wiki) = wiki();
    let report = rt.block_on(import_sources(&**wiki, &sources, Duration::from_secs(300)));
    let origin = [("adr".to_owned(), "de".to_owned())].into();
    let index = rt
        .block_on(
            PageIndex::load(Path::new("/nonexistent/none.pages"), origin)
                .unwrap()
                .link(wiki.texts().as_ref(), &variants),
        )
        .unwrap();
    let pages = index.pages.len();
    let both = index
        .pages
        .values()
        .filter(|p| p.variants.len() > 1)
        .count();
    let count = |lang: &str, group: &str| {
        index
            .pages
            .values()
            .filter(|p| p.group == group && p.variants.len() == 1 && p.variants.contains_key(lang))
            .count()
    };
    let total: usize = index.pages.values().map(|p| p.variants.len()).sum();
    let outdated = index
        .pages
        .values()
        .flat_map(|p| p.variants.keys().map(move |l| (p, l)))
        .filter(|(p, l)| index.outdated(p, l))
        .count();
    // Every variant back to its own file.
    let (mut same, mut different) = (0, Vec::new());
    for p in index.pages.values() {
        for (lang, v) in &p.variants {
            let (_, folder, ..) = config
                .iter()
                .find(|(pre, _, g, l)| {
                    *g == p.group && l == lang && v.path.starts_with(&format!("{pre}/"))
                })
                .unwrap();
            let want = std::fs::read(root.join(folder).join(&p.key)).unwrap();
            let got = rt.block_on(wiki.export_text(v.text, MARKDOWN)).unwrap();
            if got.as_bytes() == want.as_slice() {
                same += 1;
            } else {
                different.push(v.path.clone());
            }
        }
    }
    println!(
        "pages: {pages}, in two languages: {both}, only en: {} (docs) + {} (research), only de: {} (docs) + {} (adr), variants: {total}, outdated at first sight: {outdated}, byte-identical files: {same}, different: {}, failures: {}",
        count("en", "docs"),
        count("en", "research"),
        count("de", "docs"),
        count("de", "adr"),
        different.len(),
        report.failures.len()
    );
    for d in different.iter().take(20) {
        println!("different {d}");
    }
    assert!(different.is_empty() && report.failures.is_empty() && same == total);
}
