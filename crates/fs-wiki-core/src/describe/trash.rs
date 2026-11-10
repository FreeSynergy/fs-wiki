//! `TrashEntry`: a page in the trash (`TrashItem`).
//!
//! Pattern: Value object. Standard: W3C ActivityStreams 2.0 `Tombstone` (an object that was deleted, with when and who);
//! its id is the id of the page it was.

use fs_types::{BasicType as B, CommandKind as K, DescribesType, TypeDescription};

use super::{command, field, reference, PROGRAM};
use crate::TrashItem;

const T: &str = "trash";

fn describe() -> TypeDescription {
    let id = || reference("id", "TrashEntry", "wiki.trash.id");
    TypeDescription::new("TrashEntry", PROGRAM, "W3C ActivityStreams 2.0 Tombstone", "wiki.trash")
        .field(reference("id", "TrashEntry", "wiki.trash.id"))
        .field(field(T, "path", B::Text).required())
        .field(field(T, "name", B::Text))
        .field(field(T, "deletedAt", B::DateTime))
        .field(field(T, "deletedBy", B::Text))
        .field(field(T, "size", B::Text))
        .command(command(T, "find", K::Find).input(field(T, "term", B::Text)))
        .command(command(T, "restore", K::Other).input(id()).input(field(T, "path", B::Text)))
        .command(command(T, "purge", K::Delete).input(id()))
}

impl DescribesType for TrashItem {
    fn describe() -> TypeDescription {
        describe()
    }
}
