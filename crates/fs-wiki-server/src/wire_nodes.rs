//! Nodes of the tree on the wire.
//!
//! Pattern: Adapter — `Node` and `Placement` between the core's values and `texts.capnp`; an id that
//! is all zero is "none" (`Option`), a placement or kind this server does not know is refused.

use fs_interfaces::common_capnp::uuid;
use fs_interfaces::rpc::{read_object_id, read_uuid, write_object_id, write_uuid};
use fs_interfaces::texts_capnp::{node, NodeKind as Wire, Placement as WirePlacement};
use fs_objects::ObjectId;
use fs_wiki_core::{Node, NodeKind, Placement};

use crate::errors::invalid;
use crate::wire_history::ROW_BYTES;

/// A zero id is none; any other must be an object id.
pub(crate) fn optional_id(r: uuid::Reader<'_>) -> capnp::Result<Option<ObjectId>> {
    read_uuid(r).map(|_| read_object_id(r)).transpose()
}

pub(crate) fn read_placement(
    p: Result<WirePlacement, capnp::NotInSchema>,
) -> capnp::Result<Placement> {
    match p {
        Ok(WirePlacement::First) => Ok(Placement::First),
        Ok(WirePlacement::Last) => Ok(Placement::Last),
        Ok(WirePlacement::Before) => Ok(Placement::Before),
        Ok(WirePlacement::After) => Ok(Placement::After),
        _ => Err(invalid(
            "a placement is first, last, before or after",
            "placement",
        )),
    }
}

pub(crate) fn write_node(mut out: node::Builder<'_>, n: &Node) {
    write_object_id(out.reborrow().init_id(), n.id);
    out.set_kind(match n.kind {
        NodeKind::Page => Wire::Page,
        NodeKind::Folder => Wire::Folder,
        NodeKind::Space => Wire::Space,
    });
    out.set_name(n.name.as_str());
    out.set_path(n.path.as_str());
    write_uuid(out.reborrow().init_parent(), n.parent.map(|p| p.as_uuid()));
    out.set_position(n.position);
    out.set_template(n.template);
    out.set_redirect(n.redirect);
}

pub(crate) fn node_weight(n: &Node) -> usize {
    ROW_BYTES + n.name.len() + n.path.len()
}
