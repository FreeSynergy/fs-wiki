//! Search results: snippets with highlights, their limits, and the order of hits and pages.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::wiki;
use fs_wiki_core::{SearchHit, SearchQuery, Texts, WikiRepo, MAX_HIGHLIGHTS, MAX_SNIPPET_BYTES};

fn q(term: &str) -> SearchQuery {
    SearchQuery {
        term: term.into(),
        ..Default::default()
    }
}

async fn hits(w: &WikiRepo, query: &SearchQuery) -> Vec<SearchHit> {
    w.search(query, None, 50).await.unwrap().items
}

#[tokio::test]
async fn snippets_mark_the_matches_in_bytes_on_character_boundaries() {
    let w = wiki().await;
    w.create(
        "guide/tea",
        "# Tea time\n\nÄpfel und Tee: the tea is ready, TEA for all.\n",
    )
    .await
    .unwrap();
    let hit = &hits(&w, &q("tea")).await[0];
    let fields: Vec<&str> = hit.snippets.iter().map(|s| s.field).collect();
    assert_eq!(fields, ["name", "path", "body"]);
    for s in &hit.snippets {
        for h in &s.highlights {
            let marked = &s.text[h.start as usize..h.end as usize];
            assert_eq!(marked.to_lowercase(), "tea", "{s:?}");
        }
    }
    let body = hit.snippets.iter().find(|s| s.field == "body").unwrap();
    assert_eq!(body.highlights.len(), 2, "the block with the most matches");
    assert!(body.highlights.windows(2).all(|p| p[0].end <= p[1].start));
    assert!(!body.cut_before && !body.cut_after);
}

#[tokio::test]
async fn long_text_is_cut_around_the_match_within_the_limits() {
    let w = wiki().await;
    let filler = "äöü ".repeat(200);
    let body = format!("{filler}needle {} end {filler}\n", "needle ".repeat(40));
    w.create("long", &body).await.unwrap();
    let hit = &hits(&w, &q("needle")).await[0];
    let s = hit.snippets.iter().find(|s| s.field == "body").unwrap();
    assert!(s.text.len() <= MAX_SNIPPET_BYTES, "{}", s.text.len());
    assert!(s.highlights.len() <= MAX_HIGHLIGHTS && !s.highlights.is_empty());
    assert!(s.cut_before && s.cut_after);
    for h in &s.highlights {
        assert_eq!(&s.text[h.start as usize..h.end as usize], "needle");
    }
    assert!(hit.snippets.len() <= 3);
}

#[tokio::test]
async fn hits_come_by_relevance_then_newest_then_id_and_pages_continue() {
    let w = wiki().await;
    let plain = w
        .create("p", "# Notes\n\nmentions kiwi once\n")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    let named = w.create("n", "# Kiwi\n\nabout kiwi\n").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;
    let newer = w
        .create("q", "# Notes\n\nmentions kiwi once\n")
        .await
        .unwrap();
    let all = hits(&w, &q("kiwi")).await;
    let ids: Vec<_> = all.iter().map(|h| h.id).collect();
    assert_eq!(
        ids,
        [named.id, newer.id, plain.id],
        "name beats body; equal: the newer first"
    );
    let first = w.search(&q("kiwi"), None, 2).await.unwrap();
    assert!(first.more);
    let rest = w
        .search(&q("kiwi"), Some(&first.items[1].place()), 2)
        .await
        .unwrap();
    assert_eq!(
        rest.items.iter().map(|h| h.id).collect::<Vec<_>>(),
        [plain.id]
    );
    assert!(!rest.more);
}
