//! Revisions, the trash and the expectation of a write on the wire.
//!
//! Pattern: Adapter — `RevisionInfo`, `TrashItem` and `Expect` between the core's values and
//! `texts.capnp`. Sizes are small by the limits of the contract (summary, path); a document read
//! back is checked against the message size.

use fs_interfaces::rpc::{to_millis, write_object_id};
use fs_interfaces::texts_capnp::{expect, revision_info, trash_item};
use fs_wiki_core::{Expect, RevisionInfo, TrashItem};

/// Bytes a revision or trash item weighs beyond its texts.
pub(crate) const ROW_BYTES: usize = 128;

pub(crate) fn read_expect(r: expect::Reader<'_>) -> capnp::Result<Expect> {
    Ok(match r.which()? {
        expect::Which::Any(()) => Expect::Any,
        expect::Which::Revision(n) => Expect::Revision(n),
    })
}

pub(crate) fn write_info(mut out: revision_info::Builder<'_>, i: &RevisionInfo) {
    write_object_id(out.reborrow().init_text(), i.text);
    out.set_number(i.number);
    out.set_revision(i.revision);
    out.set_author(i.author.as_str());
    out.reborrow().init_at().set_unix_millis(to_millis(i.at));
    out.set_size(i.size);
    out.set_summary(i.summary.as_str());
    out.set_path(i.path.as_str());
    out.set_restored_from(i.restored_from);
}

pub(crate) fn info_weight(i: &RevisionInfo) -> usize {
    ROW_BYTES + i.author.len() + i.summary.len() + i.path.len()
}

pub(crate) fn write_trash_item(mut out: trash_item::Builder<'_>, t: &TrashItem) {
    write_object_id(out.reborrow().init_id(), t.id);
    out.set_path(t.path.as_str());
    out.set_name(t.name.as_str());
    out.reborrow()
        .init_deleted_at()
        .set_unix_millis(to_millis(t.deleted_at));
    out.set_deleted_by(t.deleted_by.as_str());
    out.set_size(t.size);
}

pub(crate) fn trash_weight(t: &TrashItem) -> usize {
    ROW_BYTES + t.path.len() + t.name.len() + t.deleted_by.len()
}
