//! The cursors of the texts 0.2 pages.
//!
//! Pattern: Adapter — bytes a client hands back unchanged. A form byte, then the place the page
//! continues after: a number (history: continue below a revision number; children: skip this many),
//! a place in the trash (milliseconds, id) or in a search result (relevance, milliseconds, id). The
//! form byte differs from the `list` / `find` cursor (2), so a cursor of one method is refused by another.

use fs_interfaces::rpc::invalid_request;
use fs_objects::ObjectId;
use fs_wiki_core::{SearchPlace, TrashPlace};

const NUMBER: u8 = 3;
const OFFSET: u8 = 6;
const TRASH: u8 = 4;
const SEARCH: u8 = 5;

fn bad() -> capnp::Error {
    invalid_request("invalid cursor")
}

/// The part of `cursor` behind its form byte, if the form is `form`; empty cursor = none.
fn body(cursor: &[u8], form: u8) -> capnp::Result<Option<&[u8]>> {
    match cursor.split_first() {
        None => Ok(None),
        Some((f, rest)) if *f == form => Ok(Some(rest)),
        Some(_) => Err(bad()),
    }
}

fn with_form(form: u8, n: u64) -> Vec<u8> {
    [&[form][..], &n.to_be_bytes()].concat()
}

fn read_form(cursor: &[u8], form: u8) -> capnp::Result<Option<u64>> {
    body(cursor, form)?
        .map(|b| Ok(u64::from_be_bytes(b.try_into().map_err(|_| bad())?)))
        .transpose()
}

/// History: continue below this revision number.
pub(crate) fn number(n: u64) -> Vec<u8> {
    with_form(NUMBER, n)
}

pub(crate) fn read_number(cursor: &[u8]) -> capnp::Result<Option<u64>> {
    read_form(cursor, NUMBER)
}

/// Children: skip this many.
pub(crate) fn offset(n: u64) -> Vec<u8> {
    with_form(OFFSET, n)
}

pub(crate) fn read_offset(cursor: &[u8]) -> capnp::Result<Option<u64>> {
    read_form(cursor, OFFSET)
}

fn id_of(bytes: &[u8]) -> capnp::Result<ObjectId> {
    let uuid = uuid::Uuid::from_slice(bytes).map_err(|_| bad())?;
    ObjectId::from_uuid(uuid).map_err(|_| bad())
}

pub(crate) fn trash(p: &TrashPlace) -> Vec<u8> {
    [
        &[TRASH][..],
        &p.deleted_ms.to_be_bytes(),
        p.id.as_uuid().as_bytes(),
    ]
    .concat()
}

pub(crate) fn read_trash(cursor: &[u8]) -> capnp::Result<Option<TrashPlace>> {
    body(cursor, TRASH)?
        .map(|b| {
            let (ms, id) = b.split_first_chunk::<8>().ok_or_else(bad)?;
            Ok(TrashPlace {
                deleted_ms: i64::from_be_bytes(*ms),
                id: id_of(id)?,
            })
        })
        .transpose()
}

pub(crate) fn search(p: &SearchPlace) -> Vec<u8> {
    [
        &[SEARCH][..],
        &p.relevance.to_be_bytes(),
        &p.updated_ms.to_be_bytes(),
        p.id.as_uuid().as_bytes(),
    ]
    .concat()
}

pub(crate) fn read_search(cursor: &[u8]) -> capnp::Result<Option<SearchPlace>> {
    body(cursor, SEARCH)?
        .map(|b| {
            let (rel, rest) = b.split_first_chunk::<4>().ok_or_else(bad)?;
            let (ms, id) = rest.split_first_chunk::<8>().ok_or_else(bad)?;
            Ok(SearchPlace {
                relevance: u32::from_be_bytes(*rel),
                updated_ms: i64::from_be_bytes(*ms),
                id: id_of(id)?,
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cursor_comes_back_and_a_foreign_one_is_refused() {
        assert_eq!(read_number(&number(7)).unwrap(), Some(7));
        assert_eq!(read_number(&[]).unwrap(), None);
        let t = TrashPlace {
            deleted_ms: -5,
            id: ObjectId::new(),
        };
        assert_eq!(read_trash(&trash(&t)).unwrap(), Some(t));
        let s = SearchPlace {
            relevance: 9,
            updated_ms: 12,
            id: ObjectId::new(),
        };
        assert_eq!(read_search(&search(&s)).unwrap(), Some(s));
        assert!(read_number(&trash(&t)).is_err());
        assert!(read_number(&offset(1)).is_err() && read_offset(&number(1)).is_err());
        assert!(read_search(&number(1)).is_err());
        assert!(read_trash(&[TRASH, 1, 2]).is_err());
    }
}
