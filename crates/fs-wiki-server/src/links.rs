//! Link rows on the wire.
//!
//! Pattern: Adapter — `LinkEntry` to `LinkRow`. The contract's target is "the readable path of the
//! target", so only links into the wiki are rows; a link to a URL or a mail address is not a text
//! and has no row. The target stays as written (it may carry a `#fragment`). The list has no cursor,
//! so an answer that would not fit one message is refused as too large.

use fs_interfaces::rpc::{check_answer, write_object_id};
use fs_interfaces::texts_capnp::link_row;
use fs_wiki_core::LinkEntry;

use crate::wire::count;

/// Bytes a row weighs beyond its two texts (the id and the framing).
const ROW_BYTES: usize = 64;

pub(crate) fn write_links(
    out: capnp::struct_list::Builder<'_, link_row::Owned>,
    rows: &[&LinkEntry],
) {
    let mut out = out;
    for (i, row) in (0u32..).zip(rows) {
        let mut r = out.reborrow().get(i);
        write_object_id(r.reborrow().init_source(), row.source);
        r.reborrow().set_target(row.target.as_str());
        r.set_at(row.at.as_str());
    }
}

/// The rows of the links into the wiki, or a refusal if they would not fit an answer.
pub(crate) fn internal(links: &[LinkEntry]) -> capnp::Result<Vec<&LinkEntry>> {
    let rows: Vec<_> = links.iter().filter(|l| l.internal).collect();
    check_answer(
        rows.iter()
            .map(|r| r.target.len() + r.at.len() + ROW_BYTES)
            .sum(),
    )?;
    count(rows.len())?;
    Ok(rows)
}
