//! German and English are ONE page with two variants: grouping, outdated translations, and the reader's language bar,
//! fallback notice and one hit per page.
//!
//! Pattern: Integration tests over a real in-memory wiki and real temp folders.

mod common;

use std::collections::HashMap;
use std::time::Duration;

use common::{temp_dir, wiki, write_files};
use fs_components_widgets::{Block, MarkKind};
use fs_components_workbenches::{DocumentSource, LinkTarget};
use fs_wiki_app::WikiSource;
use fs_wiki_app::import::import_sources;
use fs_wiki_app::variants::{PageIndex, VariantSource};

const LIMIT: Duration = Duration::from_secs(120);

fn variant(prefix: &str, group: &str, lang: &str) -> VariantSource {
    VariantSource {
        prefix: prefix.into(),
        group: group.into(),
        lang: lang.into(),
    }
}

/// The sentences of the notices and the links of the language bar of a document.
fn top(doc: &fs_components_widgets::Document) -> (Vec<String>, Vec<String>) {
    let (mut said, mut links) = (Vec::new(), Vec::new());
    for b in &doc.blocks {
        match b {
            Block::Quote(inner) => {
                for i in inner {
                    if let Block::Paragraph(p) = i {
                        said.push(p.text.clone());
                    }
                }
            }
            Block::Paragraph(p) => {
                for m in &p.marks {
                    if let MarkKind::Link { href } = &m.kind
                        && href.starts_with("?lang=")
                    {
                        links.push(href.clone());
                    }
                }
            }
            _ => {}
        }
    }
    (said, links)
}

/// The reader over the pages: one entry per page, the bar, the notices, switching, one hit per page.
fn check_reader(rt: &tokio::runtime::Runtime, wiki: &common::Fixture, index: &PageIndex) {
    // The reader: one entry per page, the bar, the notices.
    let source = WikiSource::from_report(
        wiki.texts(),
        rt.handle().clone(),
        fs_wiki_app::Report::default(),
    )
    .read_only()
    .with_pages(index.clone(), "de");
    let paths: Vec<String> = source
        .entries("")
        .unwrap()
        .into_iter()
        .map(|e| e.path)
        .collect();
    assert_eq!(paths.len(), 5, "{paths:?}");
    assert!(paths.contains(&"docs/a.md".to_owned()) && !paths.iter().any(|p| p.starts_with("en/")));
    let titles: Vec<String> = source
        .entries("docs/a")
        .unwrap()
        .into_iter()
        .map(|e| e.title)
        .collect();
    assert_eq!(
        titles,
        ["A de"],
        "the title is the one of the language read in"
    );
    let (said, links) = top(&source.read("docs/a.md").unwrap());
    assert_eq!(
        links,
        ["?lang=en"],
        "the other language is a link, the shown one is not"
    );
    assert!(said.iter().any(|s| s.contains("veraltet")), "{said:?}");
    let (said, _) = top(&source.read("docs/a.md@en").unwrap());
    assert!(said.is_empty(), "the origin is never outdated: {said:?}");
    let (said, links) = top(&source.read("docs/only-en.md").unwrap());
    assert_eq!(said, ["Diese Seite gibt es nur auf Englisch."]);
    assert!(links.is_empty(), "one variant, no bar");
    // Switching: the link opens the page in that language and the next page follows.
    assert_eq!(
        source.resolve("docs/a.md", "?lang=en"),
        LinkTarget::Document("docs/a.md@en".into())
    );
    let (said, _) = top(&source.read("docs/only-de.md").unwrap());
    assert!(
        said.iter().any(|s| s.contains("only available")),
        "now read in English: {said:?}"
    );
    // A link between the texts (`only-en.md` written next to `en/a.md`) leads to the page.
    assert_eq!(
        source.resolve("docs/a.md@en", "only-en.md"),
        LinkTarget::Document("docs/only-en.md".into())
    );
    // One hit per page, found in either language.
    assert_eq!(source.find("one").unwrap().len(), 1);
}

#[test]
fn en_and_de_are_one_page_and_an_older_translation_is_outdated() {
    let (rt, wiki) = wiki();
    let (en, de, adr) = (temp_dir("v-en"), temp_dir("v-de"), temp_dir("v-adr"));
    write_files(
        &en,
        &[
            ("a.md", "# A\n\none\n"),
            ("only-en.md", "# Only\n"),
            ("index.md", "# Home\n"),
        ],
    );
    write_files(
        &de,
        &[
            ("a.md", "# A de\n\neins\n"),
            ("only-de.md", "# Nur\n"),
            ("index.md", "# Start\n"),
        ],
    );
    write_files(&adr, &[("0001-x.md", "# 0001\n")]);
    let sources = vec![
        ("en".to_owned(), en.clone()),
        ("de".to_owned(), de.clone()),
        ("adr".to_owned(), adr.clone()),
    ];
    let variants = vec![
        variant("en", "docs", "en"),
        variant("de", "docs", "de"),
        variant("adr", "adr", "de"),
    ];
    let origin: HashMap<String, String> = [("adr".to_owned(), "de".to_owned())].into();
    assert_eq!(
        rt.block_on(import_sources(&**wiki, &sources, LIMIT))
            .imported,
        7
    );
    let file = temp_dir("v-file").join("w.pages");
    let index = PageIndex::load(&file, origin.clone()).unwrap();
    let index = rt
        .block_on(index.link(wiki.texts().as_ref(), &variants))
        .unwrap();
    let both = index
        .pages
        .values()
        .filter(|p| p.variants.len() == 2)
        .count();
    // docs/a.md and docs/index.md in two languages; docs/only-en.md, docs/only-de.md and adr/0001-x.md in one.
    assert_eq!((index.pages.len(), both), (5, 2));
    assert_eq!(index.pages["adr/0001-x.md"].variants.len(), 1);
    let id = index.pages["docs/a.md"].id;
    index.save(&file).unwrap();
    // A translation made at the first sight is in step with its origin.
    assert!(!index.outdated(&index.pages["docs/a.md"], "de"));

    // The origin changes, the translation does not: it is outdated, and the page keeps its id (also from the saved file).
    std::fs::write(en.join("a.md"), "# A\n\none and two\n").unwrap();
    assert_eq!(
        rt.block_on(import_sources(&**wiki, &sources, LIMIT))
            .updated,
        1
    );
    let index = PageIndex::load(&file, origin.clone()).unwrap();
    let index = rt
        .block_on(index.link(wiki.texts().as_ref(), &variants))
        .unwrap();
    assert_eq!(index.pages["docs/a.md"].id, id);
    assert!(index.outdated(&index.pages["docs/a.md"], "de"));
    assert!(!index.outdated(&index.pages["docs/a.md"], "en"));
    assert!(!index.outdated(&index.pages["docs/index.md"], "de"));
    index.save(&file).unwrap();

    check_reader(&rt, &wiki, &index);
    for d in [en, de, adr] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_page_with_a_component_block_comes_back_byte_for_byte_in_both_languages() {
    use fs_wiki_core::{MARKDOWN, Texts};
    let (rt, wiki) = wiki();
    let (en, de) = (temp_dir("v-comp-en"), temp_dir("v-comp-de"));
    let component = |fallback: &str| {
        let spec = fs_text::ComponentSpec {
            component: "fs.tasks.board".into(),
            version: 1,
            config: serde_json::json!({"columns": ["todo", "done"], "note": "a -- b"}),
            source: Some(fs_text::DataSource {
                setting: "tasks.board.project".into(),
                params: serde_json::Value::Null,
            }),
        };
        format!(
            "# Board\n\nText before.\n\n{}\nText after.\n",
            spec.markdown(fallback)
        )
    };
    write_files(
        &en,
        &[(
            "board.md",
            &component("| Task | State |\n|---|---|\n| Write | done |"),
        )],
    );
    write_files(
        &de,
        &[(
            "board.md",
            &component("| Aufgabe | Stand |\n|---|---|\n| Schreiben | erledigt |"),
        )],
    );
    let sources = vec![("en".to_owned(), en.clone()), ("de".to_owned(), de.clone())];
    assert_eq!(
        rt.block_on(import_sources(&**wiki, &sources, LIMIT))
            .imported,
        2
    );
    for (prefix, dir) in &sources {
        let want = std::fs::read(dir.join("board.md")).unwrap();
        let entry = rt
            .block_on(wiki.get_by_path(&format!("{prefix}/board.md")))
            .unwrap()
            .unwrap();
        let got = rt.block_on(wiki.export_text(entry.id, MARKDOWN)).unwrap();
        assert_eq!(got.as_bytes(), want.as_slice(), "{prefix}");
        let (found, problems) = fs_text::find_components(&entry.document);
        assert!(
            problems.is_empty() && found.len() == 1,
            "{prefix}: {problems:?}"
        );
        assert_eq!(found[0].spec.component, "fs.tasks.board");
    }
    // One page, two variants, each with its own fallback.
    let index = fs_wiki_app::variants::PageIndex::default();
    let index = rt
        .block_on(index.link(
            wiki.texts().as_ref(),
            &[variant("en", "docs", "en"), variant("de", "docs", "de")],
        ))
        .unwrap();
    assert_eq!(index.pages["docs/board.md"].variants.len(), 2);
    for d in [en, de] {
        std::fs::remove_dir_all(d).unwrap();
    }
}
