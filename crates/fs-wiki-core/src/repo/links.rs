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
use crate::error::{Error, Result};
use crate::path::page_of;
use crate::texts::{LinkEntry, Paging, TextEntry};

fn rows_of(text: &TextEntry) -> Vec<LinkEntry> {
    extract_links(&text.document)
        .into_iter()
        .map(|r| LinkEntry {
            source: text.id,
            target: r.path,
            at: format!("{}:{}-{}", r.source_block, r.start, r.end),
            title: r.title,
            internal: r.internal,
        })
        .collect()
}

impl WikiRepo {
    pub(super) async fn links_of(&self, id: ObjectId) -> Result<Vec<LinkEntry>> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Ok(rows_of(&text))
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
                    .filter(|r| r.internal && page_of(&r.target) == page),
            );
        }
        Ok(rows)
    }
}
