//! Search: whole words, snippets, filters, a stable order.
//!
//! Pattern: Pipeline — candidates from the store (narrowed by the longest word when it is ASCII, the
//! path prefix when given), then the exact checks here: every word in the name, the path or a block
//! (whole words, case-insensitive; the last as a prefix if asked), the filters, a relevance, and
//! the order (relevance descending, updated descending, id ascending). Candidates are read whole:
//! the cost is the texts the store cannot exclude, not the page.

use std::cmp::Ordering;

use fs_objects::{Cond, Op};

use super::handle::WikiRepo;
use super::read::all_texts;
use super::record::entry_of;
use super::snippets::snippet;
use super::words::{query_words, Needles};
use crate::error::{Error, Result};
use crate::hidden::is_hidden_block;
use crate::limits::{
    MAX_SEARCH_PAGE, MAX_SEARCH_TERM_BYTES, MAX_SEARCH_WORDS, MAX_SNIPPETS_PER_HIT,
    MIN_PREFIX_BYTES,
};
use crate::search::{SearchFilter, SearchHit, SearchPage, SearchPlace, SearchQuery, Snippet, Tri};
use crate::texts::{Paging, TextEntry};

fn invalid(issue: &'static str, detail: String) -> Error {
    Error::Invalid { issue, detail }
}

fn needles(query: &SearchQuery) -> Result<Needles> {
    if query.term.len() > MAX_SEARCH_TERM_BYTES {
        return Err(invalid(
            "term-too-long",
            format!("a term is at most {MAX_SEARCH_TERM_BYTES} bytes"),
        ));
    }
    let words = query_words(&query.term);
    if words.len() > MAX_SEARCH_WORDS {
        return Err(invalid(
            "too-many-words",
            format!("a term has at most {MAX_SEARCH_WORDS} words"),
        ));
    }
    if query.prefix && words.last().is_some_and(|w| w.len() < MIN_PREFIX_BYTES) {
        return Err(invalid(
            "prefix-too-short",
            format!("a prefix is at least {MIN_PREFIX_BYTES} bytes"),
        ));
    }
    Ok(Needles {
        words,
        prefix: query.prefix,
    })
}

fn tri(rule: Tri, value: bool) -> bool {
    match rule {
        Tri::Any => true,
        Tri::Only => value,
        Tri::Exclude => !value,
    }
}

fn allows(f: &SearchFilter, space_prefix: Option<&str>, t: &TextEntry) -> bool {
    let at = t.envelope.updated.timestamp_millis();
    tri(f.template, t.template)
        && tri(f.redirect, !t.redirect_to.is_empty())
        && t.path.starts_with(&f.path_prefix)
        && space_prefix.is_none_or(|p| t.path.starts_with(p))
        && (f.author.is_empty() || t.updated_by == f.author)
        && (f.updated_from_ms == 0 || at >= f.updated_from_ms)
        && (f.updated_to_ms == 0 || at < f.updated_to_ms)
}

/// The blocks of `t` a search reads: the ones `find` reads.
fn blocks(t: &TextEntry) -> Vec<String> {
    t.document
        .blocks
        .iter()
        .filter(|b| !is_hidden_block(b))
        .map(|b| b.text.to_string())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// The hit for `t`, or none if a word is missing everywhere.
fn hit_of(t: &TextEntry, n: &Needles) -> Option<SearchHit> {
    let mut hit = SearchHit {
        id: t.id,
        path: t.path.clone(),
        name: t.envelope.name.clone(),
        updated: t.envelope.updated,
        updated_by: t.updated_by.clone(),
        template: t.template,
        redirect: !t.redirect_to.is_empty(),
        snippets: vec![],
        relevance: 0,
    };
    if n.words.is_empty() {
        return Some(hit);
    }
    let body = blocks(t);
    let all = body.join("\n");
    // Every word must occur somewhere.
    let covered = (0..n.words.len()).all(|i| {
        let one = n.only(i);
        one.hits(&hit.name) + one.hits(&hit.path) + one.hits(&all) > 0
    });
    if !covered {
        return None;
    }
    let fields: [(&'static str, Vec<&str>, usize); 3] = [
        ("name", vec![hit.name.as_str()], 3),
        ("path", vec![hit.path.as_str()], 2),
        ("body", body.iter().map(String::as_str).collect(), 1),
    ];
    let mut snippets: Vec<Snippet> = vec![];
    let mut relevance = 0;
    for (field, texts, weight) in fields {
        relevance += weight * texts.iter().map(|x| n.hits(x)).max().unwrap_or(0);
        // The text with the most matches (the first of equals) is the one shown.
        let mut best: Option<(&str, Vec<(usize, usize)>)> = None;
        for x in &texts {
            let m = n.matches(x);
            if m.len() > best.as_ref().map_or(0, |b| b.1.len()) {
                best = Some((x, m));
            }
        }
        snippets.extend(best.and_then(|(x, m)| snippet(field, x, &m)));
    }
    snippets.truncate(MAX_SNIPPETS_PER_HIT);
    hit.snippets = snippets;
    hit.relevance = u32::try_from(relevance).unwrap_or(u32::MAX);
    Some(hit)
}

/// The order of a result: relevance descending, updated descending, id ascending.
fn order(a: &SearchPlace, b: &SearchPlace) -> Ordering {
    b.relevance
        .cmp(&a.relevance)
        .then(b.updated_ms.cmp(&a.updated_ms))
        .then(a.id.cmp(&b.id))
}

impl WikiRepo {
    pub(super) async fn search_page(
        &self,
        query: &SearchQuery,
        after: Option<&SearchPlace>,
        size: usize,
    ) -> Result<SearchPage> {
        let n = needles(query)?;
        let f = &query.filter;
        let space = match f.space {
            Some(id) => Some(format!("{}/", self.folder(id).await?.path)),
            None => None,
        };
        let mut q = all_texts(Paging::default());
        if !f.path_prefix.is_empty() {
            q = q.index_between(
                "path",
                f.path_prefix.clone(),
                format!("{}\u{10FFFF}", f.path_prefix),
            );
        }
        if let Some(word) = n
            .words
            .iter()
            .filter(|w| w.is_ascii())
            .max_by_key(|w| w.len())
        {
            let contains = |key: &str| Cond::Index {
                key: key.into(),
                op: Op::Contains,
                value: word.clone(),
            };
            q.any.push(Cond::Name {
                op: Op::Contains,
                value: word.clone(),
            });
            q.any.push(contains("path"));
            q.any.push(contains("block"));
        }
        let mut hits: Vec<SearchHit> = vec![];
        for o in self.reads.query(&q).await? {
            let t = entry_of(o)?;
            if allows(f, space.as_deref(), &t) {
                hits.extend(hit_of(&t, &n));
            }
        }
        hits.sort_by(|a, b| order(&a.place(), &b.place()));
        if let Some(after) = after {
            hits.retain(|h| order(after, &h.place()).is_lt());
        }
        let size = size.clamp(1, MAX_SEARCH_PAGE);
        let more = hits.len() > size;
        hits.truncate(size);
        Ok(SearchPage { items: hits, more })
    }
}
