//! Reading texts: by id, by path, listed by path prefix, found by term.
//!
//! Pattern: Builder consumer — each read assembles one store `Query` (type, context, conditions,
//! order, page) that the store runs, so only the requested page is read into memory.

use fs_store::{Cond, ObjectId, Op, Query};

use super::handle::{WikiRepo, CONTEXT, TEXT_TYPE};
use super::record::entry_of;
use crate::error::Result;
use crate::texts::{Paging, TextEntry};

/// The query for all texts, in path order, with `page` applied.
pub(super) fn all_texts(page: Paging) -> Query {
    let mut q = Query::new()
        .of_type(TEXT_TYPE)
        .in_context(CONTEXT)
        .order_by_index("path")
        .offset(page.offset);
    q.limit = page.limit;
    q
}

impl WikiRepo {
    pub(super) async fn text(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        match self.objects.get(id).await? {
            Some(o) if o.envelope.object_type == TEXT_TYPE => entry_of(o).map(Some),
            _ => Ok(None),
        }
    }

    pub(super) async fn text_at(&self, path: &str) -> Result<Option<TextEntry>> {
        let q = all_texts(Paging::default()).with_index("path", path);
        self.reads
            .query(&q)
            .await?
            .into_iter()
            .next()
            .map(entry_of)
            .transpose()
    }

    pub(super) async fn list_texts(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        let mut q = all_texts(page);
        if !prefix.is_empty() {
            // Binary text order: every string starting with `prefix` lies in [prefix, prefix + U+10FFFF).
            q = q.index_between("path", prefix, format!("{prefix}\u{10FFFF}"));
        }
        self.reads
            .query(&q)
            .await?
            .into_iter()
            .map(entry_of)
            .collect()
    }

    pub(super) async fn find_texts(&self, term: &str, page: Paging) -> Result<Vec<TextEntry>> {
        let mut q = all_texts(page);
        let contains = |key: &str| Cond::Index {
            key: key.into(),
            op: Op::Contains,
            value: term.into(),
        };
        q.any.push(Cond::Name {
            op: Op::Contains,
            value: term.into(),
        });
        q.any.push(contains("path"));
        q.any.push(contains("block"));
        self.reads
            .query(&q)
            .await?
            .into_iter()
            .map(entry_of)
            .collect()
    }
}
