//! One rule for links: `[[path]]` is a path from the root in the core, so the reader must open it
//! from the root too, wherever the page stands.
//!
//! Pattern: Integration test over a real in-memory wiki and the reader's data source.

mod common;

use common::wiki;
use fs_components_widgets::{Block, MarkKind};
use fs_components_workbenches::{DocumentSource, LinkTarget};
use fs_wiki_app::WikiSource;
use fs_wiki_core::Texts;

fn hrefs(doc: &fs_components_widgets::Document) -> Vec<String> {
    let mut out = vec![];
    for b in &doc.blocks {
        if let Block::Paragraph(p) = b {
            for m in &p.marks {
                if let MarkKind::Link { href } = &m.kind {
                    out.push(href.clone());
                }
            }
        }
    }
    out
}

#[test]
fn a_wiki_link_opens_the_page_the_core_means_from_any_folder() {
    let (rt, w) = wiki();
    rt.block_on(async {
        w.create("deep/er/c/z.md", "z").await.unwrap();
        w.create("deep/er/c/q.md", "[near](z.md)\n").await.unwrap();
        w.create(
            "x/y/page.md",
            "See [[deep/er/c/z.md]] and [[deep/er/c/z.md#h|label]].\n",
        )
        .await
        .unwrap();
    });
    let src = WikiSource::new(w, rt.handle().clone(), vec![]);
    let found = hrefs(&src.read("x/y/page.md").unwrap());
    assert_eq!(found.len(), 2);
    for href in found {
        assert_eq!(
            src.resolve("x/y/page.md", &href),
            LinkTarget::Document("deep/er/c/z.md".into()),
            "{href}"
        );
    }
    // A Markdown link stays relative to its page.
    let near = hrefs(&src.read("deep/er/c/q.md").unwrap());
    assert_eq!(
        src.resolve("deep/er/c/q.md", &near[0]),
        LinkTarget::Document("deep/er/c/z.md".into())
    );
}
