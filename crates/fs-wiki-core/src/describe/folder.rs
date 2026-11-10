//! `Folder`: a space or a folder of pages (`Node` of kind space or folder).
//!
//! Pattern: Value object. Standard: W3C ActivityStreams 2.0 `OrderedCollection`, because a folder holds pages in an order
//! the person sets (texts.capnp `reorder`); the pages inside are the `Page` records whose path starts with the folder's.

use fs_types::{BasicType as B, CommandKind as K, DescribesType, TypeDescription};

use super::{command, field, optional_reference, reference, PROGRAM};
use crate::Node;

const T: &str = "folder";

fn describe() -> TypeDescription {
    let id = || reference("id", "Folder", "wiki.folder.id");
    TypeDescription::new("Folder", PROGRAM, "W3C ActivityStreams 2.0 OrderedCollection", "wiki.folder")
        .field(reference("id", "Folder", "wiki.folder.id"))
        .field(field(T, "kind", B::Choice).choices(&["space", "folder"]).required())
        .field(field(T, "name", B::Text).required())
        .field(field(T, "path", B::Text).required())
        .field(optional_reference("parent", "Folder", "wiki.folder.parent"))
        .field(field(T, "position", B::Text))
        .command(command(T, "find", K::Find).input(field(T, "name", B::Text)).input(optional_reference("parent", "Folder", "wiki.folder.parent")))
        .command(
            command(T, "create", K::Create)
                .input(optional_reference("parent", "Folder", "wiki.folder.parent"))
                .input(field(T, "name", B::Text).required())
                .input(field(T, "path", B::Text)),
        )
        .command(command(T, "rename", K::Change).input(id()).input(field(T, "name", B::Text).required()))
        .command(command(T, "move", K::Change).input(id()).input(field(T, "path", B::Text).required()))
        .command(
            command(T, "reorder", K::Change)
                .input(id())
                .input(field(T, "placement", B::Choice).choices(&["first", "last", "before", "after"]).required())
                .input(optional_reference("anchor", "Folder", "wiki.folder.anchor")),
        )
        .command(command(T, "delete", K::Delete).input(id()))
}

impl DescribesType for Node {
    fn describe() -> TypeDescription {
        describe()
    }
}
