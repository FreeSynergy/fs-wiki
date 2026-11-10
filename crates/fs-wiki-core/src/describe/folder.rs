//! `Folder`: a space or a folder of pages (`Node` of kind space or folder).
//!
//! Pattern: Value object. Standard: W3C `ActivityStreams` 2.0 `OrderedCollection`, because a folder holds pages in an order
//! the person sets (texts.capnp `reorder`); a `parent` or `anchor` in a command names no type (a space has no parent folder, an
//! anchor may be a page, and a reference to the own type would bind the command to a record in the frontend); the pages inside are the `Page` records whose path starts with the folder's.

use fs_types::{BasicType as B, CommandKind as K, DescribesType, TypeDescription};

use super::{any_reference, command, field, optional_reference, reference, PROGRAM};
use crate::Node;

const T: &str = "folder";

fn describe() -> TypeDescription {
    let id = || reference("id", "Folder", "wiki.folder.id");
    TypeDescription::new(
        "Folder",
        PROGRAM,
        "W3C ActivityStreams 2.0 OrderedCollection",
        "wiki.folder",
    )
    .field(reference("id", "Folder", "wiki.folder.id"))
    .field(
        field(T, "kind", B::Choice)
            .choices(&["space", "folder"])
            .required(),
    )
    .field(field(T, "name", B::Text).required())
    .field(field(T, "path", B::Text).required())
    .field(optional_reference("parent", "Folder", "wiki.folder.parent"))
    .field(field(T, "position", B::Text))
    .command(
        command(T, "find", K::Find)
            .input(field(T, "name", B::Text))
            .input(any_reference("parent", "wiki.folder.parent")),
    )
    .command(
        command(T, "create", K::Create)
            .input(any_reference("parent", "wiki.folder.parent"))
            .input(field(T, "name", B::Text).required())
            .input(field(T, "path", B::Text)),
    )
    .command(
        command(T, "rename", K::Change)
            .input(id())
            .input(field(T, "name", B::Text).required()),
    )
    .command(
        command(T, "move", K::Change)
            .input(id())
            .input(field(T, "path", B::Text).required()),
    )
    .command(
        command(T, "reorder", K::Change)
            .input(id())
            .input(
                field(T, "placement", B::Choice)
                    .choices(&["first", "last", "before", "after"])
                    .required(),
            )
            .input(any_reference("anchor", "wiki.folder.anchor")),
    )
    .command(command(T, "delete", K::Delete).input(id()))
}

impl DescribesType for Node {
    fn describe() -> TypeDescription {
        describe()
    }
}
