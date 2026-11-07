//! A page of texts, cut in the store at a place in the sort key.
//!
//! Pattern: Iterator (external, resumable) over the Repository — the page after a [`Place`] is
//! asked of the store as at most three queries, each with a limit, never as the whole list that is
//! cut afterwards. The order is the store's `order_by_index` of `path_key`: the smallest value
//! (already the sort key of the path), then the case-folded name, then the id as text. The
//! conditions compare the same three things the same way (`Gt` on the raw `path_key`, which holds the
//! sort key itself, so raw and folded order agree; the name through the store's own folding; the id
//! as text), so no text falls between two pages and none comes twice.

use fs_store::{Cond, Op, Query, StoredObject};

use super::handle::{WikiRepo, CONTEXT, TEXT_TYPE};
use super::keys::{BLOCK, ID_KEY, PATH, PATH_KEY};
use super::record::entry_of;
use crate::error::{Error, Result};
use crate::page::{Place, Placed, TextPage, TextQuery};

/// The store query for `query`, in page order.
fn base(query: &TextQuery) -> Query {
    let q = Query::new()
        .of_type(TEXT_TYPE)
        .in_context(CONTEXT)
        .order_by_index(PATH_KEY);
    match query {
        // Binary text order: every string starting with `prefix` lies in [prefix, prefix + U+10FFFF).
        TextQuery::Prefix(prefix) if !prefix.is_empty() => {
            q.index_between(PATH, prefix, format!("{prefix}\u{10FFFF}"))
        }
        TextQuery::Prefix(_) => q,
        TextQuery::Term(term) => {
            let contains = |key: &str| Cond::Index {
                key: key.into(),
                op: Op::Contains,
                value: term.clone(),
            };
            q.or(Cond::Name {
                op: Op::Contains,
                value: term.clone(),
            })
            .or(contains(PATH))
            .or(contains(BLOCK))
        }
    }
}

/// The queries that together give everything after `place`, in order: the rest of the same key and
/// name (by id), the later names of the same key, the later keys.
fn after(base: Query, place: &Place) -> Vec<Query> {
    let same_key = |q: Query| q.with_index(PATH_KEY, &place.key);
    let name = |op| Cond::Name {
        op,
        value: place.name.clone(),
    };
    vec![
        same_key(base.clone())
            .and(name(Op::Eq))
            .index_op(ID_KEY, Op::Gt, place.id.to_string()),
        same_key(base.clone()).and(name(Op::Gt)),
        base.index_op(PATH_KEY, Op::Gt, &place.key),
    ]
}

fn placed(object: StoredObject) -> Result<Placed> {
    let id = object.envelope.id;
    let key = object
        .index
        .get(PATH_KEY)
        .and_then(|values| values.first())
        .cloned()
        .ok_or_else(|| Error::Stored(format!("text {id} has no {PATH_KEY}; run upgrade_index")))?;
    let place = Place {
        key,
        name: object.envelope.name.clone(),
        id,
    };
    Ok(Placed {
        entry: entry_of(object)?,
        place,
    })
}

impl WikiRepo {
    /// The first page checks that every text has a place in the order. One that has not (written
    /// before 0.3.0, or not repairable) would be sorted last by the store and then be skipped by every
    /// later page; so it is said here, with the way out, and nothing is cut short unnoticed.
    async fn check_places(&self) -> Result<()> {
        let all = Query::new().of_type(TEXT_TYPE).in_context(CONTEXT);
        let placed = all.clone().index_op(PATH_KEY, Op::Ge, "");
        let (all, placed) = (self.reads.count(&all).await?, self.reads.count(&placed).await?);
        if placed < all {
            return Err(Error::Stored(format!(
                "{} of {all} texts have no place in the order (written before 0.3.0 or not repairable); run upgrade_index and read its report",
                all - placed
            )));
        }
        Ok(())
    }

    pub(super) async fn page_of(
        &self,
        query: &TextQuery,
        place: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        if place.is_none() {
            self.check_places().await?;
        }
        // One more than asked: that is how a page knows there is a next one.
        let want = size.saturating_add(1);
        let base = base(query);
        let steps = match place {
            Some(p) => after(base, p),
            None => vec![base],
        };
        let mut found: Vec<StoredObject> = Vec::new();
        for step in steps {
            let missing = want - found.len();
            if missing == 0 {
                break;
            }
            found.extend(self.reads.query(&step.limit(missing)).await?);
        }
        let more = found.len() > size;
        found.truncate(size);
        Ok(TextPage {
            items: found.into_iter().map(placed).collect::<Result<_>>()?,
            more,
        })
    }
}
