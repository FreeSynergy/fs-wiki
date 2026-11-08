//! Search: whole words, a prefix for the last one, filters and the term limits.
//! Pattern: Example-based integration tests over a real in-memory store.
mod common;

use common::wiki;
use fs_wiki_core::{Error, Expect, SearchFilter, SearchHit, SearchQuery, Texts, Tri, WikiRepo};

fn q(term: &str) -> SearchQuery {
    SearchQuery {
        term: term.into(),
        ..Default::default()
    }
}

async fn hits(w: &WikiRepo, query: &SearchQuery) -> Vec<SearchHit> {
    w.search(query, None, 50).await.unwrap().items
}

async fn paths(w: &WikiRepo, query: &SearchQuery) -> Vec<String> {
    let mut found: Vec<String> = hits(w, query).await.into_iter().map(|h| h.path).collect();
    found.sort();
    found
}

#[tokio::test]
async fn words_match_whole_ignore_case_and_all_must_match() {
    let w = wiki().await;
    w.create("a", "Bring a Towel and tea.\n").await.unwrap();
    w.create("b", "Towels are warm, tea is hot.\n")
        .await
        .unwrap();
    w.create("c", "Only tea here.\n").await.unwrap();
    assert_eq!(
        paths(&w, &q("towel")).await,
        ["a"],
        "whole words: towels is another word"
    );
    assert_eq!(
        paths(&w, &q("TEA towel")).await,
        ["a"],
        "all words, any case, any order"
    );
    assert_eq!(paths(&w, &q("tea")).await, ["a", "b", "c"]);
    assert_eq!(
        paths(&w, &q("towel zebra")).await,
        [] as [std::string::String; 0]
    );
    assert_eq!(paths(&w, &q("tow")).await, Vec::<String>::new());
    let prefix = SearchQuery {
        term: "tea tow".into(),
        prefix: true,
        ..Default::default()
    };
    assert_eq!(
        paths(&w, &prefix).await,
        ["a", "b"],
        "the last word is a prefix, the others whole"
    );
    let name_or_path = w.create("manual/Zebra-guide", "x\n").await.unwrap();
    assert_eq!(
        paths(&w, &q("zebra")).await,
        [name_or_path.path.as_str()],
        "the path counts"
    );
}

#[tokio::test]
async fn limits_of_the_term_are_invalid_requests() {
    let w = wiki().await;
    let issue = |e: Error| match e {
        Error::Invalid { issue, .. } => issue,
        other => panic!("{other:?}"),
    };
    let long = "x".repeat(257);
    assert_eq!(
        issue(w.search(&q(&long), None, 10).await.unwrap_err()),
        "term-too-long"
    );
    let nine = "a b c d e f g h i";
    assert_eq!(
        issue(w.search(&q(nine), None, 10).await.unwrap_err()),
        "too-many-words"
    );
    assert!(
        w.search(&q("a b c d e f g h"), None, 10).await.is_ok(),
        "eight words are fine"
    );
    let short = SearchQuery {
        term: "tea x".into(),
        prefix: true,
        ..Default::default()
    };
    assert_eq!(
        issue(w.search(&short, None, 10).await.unwrap_err()),
        "prefix-too-short"
    );
    let two = SearchQuery {
        term: "xy".into(),
        prefix: true,
        ..Default::default()
    };
    assert!(
        w.search(&two, None, 10).await.is_ok(),
        "two bytes are enough"
    );
    let wide = SearchQuery {
        term: "é".into(),
        prefix: true,
        ..Default::default()
    };
    assert!(w.search(&wide, None, 10).await.is_ok(), "é is two bytes");
}

#[tokio::test]
async fn filters_combine_and_an_empty_term_lists_without_snippets() {
    let w = wiki().await;
    let tpl = w.create("tpl/form", "Form text\n").await.unwrap();
    w.set_template(tpl.id, true, Expect::Any).await.unwrap();
    let old = w.create("docs/old", "Form text\n").await.unwrap();
    w.set_redirect(old.id, "docs/new", Expect::Any)
        .await
        .unwrap();
    w.create("docs/new", "Form text\n").await.unwrap();
    let with = |f: SearchFilter| SearchQuery {
        term: "form".into(),
        filter: f,
        ..Default::default()
    };
    let f = |t, r| SearchFilter {
        template: t,
        redirect: r,
        ..Default::default()
    };
    assert_eq!(paths(&w, &with(f(Tri::Only, Tri::Any))).await, ["tpl/form"]);
    assert_eq!(
        paths(&w, &with(f(Tri::Exclude, Tri::Any))).await,
        ["docs/new", "docs/old"]
    );
    assert_eq!(paths(&w, &with(f(Tri::Any, Tri::Only))).await, ["docs/old"]);
    assert_eq!(
        paths(&w, &with(f(Tri::Exclude, Tri::Exclude))).await,
        ["docs/new"]
    );
    let prefix = SearchFilter {
        path_prefix: "docs/".into(),
        ..Default::default()
    };
    assert_eq!(paths(&w, &with(prefix)).await, ["docs/new", "docs/old"]);
    let case = SearchFilter {
        path_prefix: "Docs/".into(),
        ..Default::default()
    };
    assert!(paths(&w, &with(case)).await.is_empty(), "case sensitive");
    let me = SearchFilter {
        author: common::ME.into(),
        ..Default::default()
    };
    assert_eq!(hits(&w, &with(me)).await.len(), 3);
    let other = SearchFilter {
        author: "someone".into(),
        ..Default::default()
    };
    assert_eq!(
        hits(&w, &with(other)).await,
        [] as [fs_wiki_core::SearchHit; 0]
    );
    let space = w.create_folder(None, "Docs", "docs").await.unwrap();
    let in_space = SearchFilter {
        space: Some(space.id),
        ..Default::default()
    };
    assert_eq!(paths(&w, &with(in_space)).await, ["docs/new", "docs/old"]);
    let only_filters = hits(
        &w,
        &SearchQuery {
            filter: f(Tri::Only, Tri::Any),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(only_filters.len(), 1);
    assert!(only_filters[0].snippets.is_empty() && only_filters[0].template);
    assert_eq!(hits(&w, &SearchQuery::default()).await.len(), 3);
}

#[tokio::test]
async fn the_time_filter_is_half_open() {
    let w = wiki().await;
    let a = w.create("a", "word\n").await.unwrap();
    let at = hits(&w, &q("word")).await[0].updated.timestamp_millis();
    let range = |from, to| SearchQuery {
        term: "word".into(),
        filter: SearchFilter {
            updated_from_ms: from,
            updated_to_ms: to,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(hits(&w, &range(at, 0)).await.len(), 1, "at or after");
    assert_eq!(hits(&w, &range(at + 1, 0)).await.len(), 0);
    assert_eq!(hits(&w, &range(0, at)).await.len(), 0, "before, not at");
    assert_eq!(hits(&w, &range(0, at + 1)).await.len(), 1);
    assert_eq!(hits(&w, &range(at, at + 1)).await[0].id, a.id);
}
