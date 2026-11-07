//! Link rows on the wire.
//!
//! Pattern: Adapter — `LinkEntry` to `LinkRow`. Every link is a row: `title` is the link text as
//! written, `internal` says whether the target is a text of this wiki (it may not exist yet) or an
//! outside address (URL, mail). The target stays as written (it may carry a `#fragment`). The list
//! has no cursor, so an answer that would not fit one message is refused as too large.

use fs_interfaces::rpc::{check_answer, write_object_id};
use fs_interfaces::texts_capnp::link_row;
use fs_wiki_core::LinkEntry;

use crate::wire::count;

/// Bytes a row weighs beyond its two texts (the id and the framing).
const ROW_BYTES: usize = 64;

pub(crate) fn write_links(
    out: capnp::struct_list::Builder<'_, link_row::Owned>,
    rows: &[LinkEntry],
) {
    let mut out = out;
    for (i, row) in (0u32..).zip(rows) {
        let mut r = out.reborrow().get(i);
        write_object_id(r.reborrow().init_source(), row.source);
        r.reborrow().set_target(row.target.as_str());
        r.reborrow().set_at(row.at.as_str());
        r.reborrow().set_title(row.title.as_str());
        r.set_internal(row.internal);
    }
}

/// The links as rows, or a refusal if they would not fit an answer.
pub(crate) fn rows(links: &[LinkEntry]) -> capnp::Result<&[LinkEntry]> {
    check_answer(
        links
            .iter()
            .map(|r| r.target.len() + r.at.len() + r.title.len() + ROW_BYTES)
            .sum(),
    )?;
    count(links.len())?;
    Ok(links)
}
