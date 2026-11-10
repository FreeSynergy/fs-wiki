//! `Revision`: one state of a page in its history (`RevisionInfo`).
//!
//! Pattern: Value object. Standard: RFC 7089 (HTTP Framework for Time-Based Access to Resource States, "Memento"): a
//! revision is the memento of a page at a time. Its id is `<page id>#<number>` (texts.capnp: `number` counts per text).

use fs_types::{BasicType as B, CommandKind as K, DescribesType, TypeDescription};

use super::{command, field, reference, PROGRAM};
use crate::RevisionInfo;

const T: &str = "revision";

fn describe() -> TypeDescription {
    let id = || reference("id", "Revision", "wiki.revision.id");
    TypeDescription::new("Revision", PROGRAM, "RFC 7089 Memento", "wiki.revision")
        .field(reference("id", "Revision", "wiki.revision.id"))
        .field(reference("page", "Page", "wiki.revision.page"))
        .field(field(T, "number", B::Text).required())
        .field(field(T, "revision", B::Text))
        .field(field(T, "author", B::Text))
        .field(field(T, "at", B::DateTime))
        .field(field(T, "size", B::Text))
        .field(field(T, "summary", B::Text))
        .field(field(T, "path", B::Text))
        .field(field(T, "restoredFrom", B::Text))
        .command(command(T, "find", K::Find).input(reference("page", "Page", "wiki.revision.page")))
        .command(command(T, "get", K::Other).input(id()).result(field(
            T,
            "content",
            B::MultilineText,
        )))
        .command(
            command(T, "restore", K::Other)
                .input(id())
                .input(field(T, "revision", B::Text)),
        )
        .command(command(T, "purge", K::Delete).input(id()))
}

impl DescribesType for RevisionInfo {
    fn describe() -> TypeDescription {
        describe()
    }
}
