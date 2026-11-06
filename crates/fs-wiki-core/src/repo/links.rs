//! The link table: links of one text, and back references to a path.
//!
//! Pattern: Read model — the rows are `extract_links` of the stored document (the marks are the
//! truth). Back references are one store query on the index `link` (the page part of every
//! internal link), then the rows of the texts found.

use fs_store::ObjectId;
use fs_text::extract_links;

use super::handle::WikiRepo;
use super::read::all_texts;
use super::record::entry_of;
use super::target::written;
use super::titles::{title_links, titles_of};
use crate::error::{Error, Result};
use crate::path::page_of;
use crate::texts::{LinkEntry, Paging, TextEntry};

/// The rows of `text`, each with the path it means (`None` for external links and anchors).
fn rows_of(text: &TextEntry) -> Vec<(LinkEntry, Vec<String>)> {
    extract_links(&text.document)
        .into_iter()
        .map(|r| {
            let meant = text
                .document
                .block(r.source_block)
                .and_then(|b| b.marks.get(r.mark))
                .and_then(|m| m.link())
                .and_then(|l| written(l, &text.path))
                .map(|w| w.candidates())
                .unwrap_or_default();
            let entry = LinkEntry {
                source: text.id,
                target: r.path,
                at: format!("{}:{}-{}", r.source_block, r.start, r.end),
                title: r.title,
                internal: r.internal,
            };
            (entry, meant)
        })
        .chain(title_rows(text))
        .collect()
}

impl WikiRepo {
    pub(super) async fn links_of(&self, id: ObjectId) -> Result<Vec<LinkEntry>> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Ok(rows_of(&text).into_iter().map(|(e, _)| e).collect())
    }

    pub(super) async fn backlinks_to(&self, path: &str) -> Result<Vec<LinkEntry>> {
        let page = page_of(path);
        let q = all_texts(Paging::default()).with_index("link", page);
        let mut rows = vec![];
        for o in self.reads.query(&q).await? {
            let text = entry_of(o)?;
            rows.extend(
                rows_of(&text)
                    .into_iter()
                    .filter(|(r, meant)| r.internal && meant.iter().any(|m| m == page))
                    .map(|(r, _)| r),
            );
        }
        Ok(rows)
    }
}

/// The links in callout titles (not marks, so not in `extract_links`), one row each.
fn title_rows(text: &TextEntry) -> Vec<(LinkEntry, Vec<String>)> {
    let mut rows = vec![];
    for block in &text.document.blocks {
        for title in titles_of(block) {
            for link in title_links(title) {
                let Some(w) = written(&link, &text.path) else {
                    continue;
                };
                let entry = LinkEntry {
                    source: text.id,
                    target: link.path.clone(),
                    at: format!("{}:title", block.id),
                    title: title.to_string(),
                    internal: true,
                };
                rows.push((entry, w.candidates()));
            }
        }
    }
    rows
}
