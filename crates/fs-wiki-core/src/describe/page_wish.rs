//! Commands of `Page` that `requirements.md` asks for and the core does not have yet.
//!
//! Pattern: none (plain data: a list of descriptions). They are in the description on purpose (ADR 0066, rule of
//! 2026-11-10): the frontend is built complete, a control for a command the core does not report as supported is locked
//! and says it is still to come. Which row of `requirements.md` each one is for is in the comment at the name.

use fs_types::{BasicType as B, CommandKind as K, TypeDescription};

use super::{command, field, optional_reference, reference};

const T: &str = "page";

/// The names, for the table of what the program supports.
pub(crate) const NAMES: &[&str] = &[
    "share", "unshare", "comment", "openTalkPage", "attach", "detach", "setLabels", "watch", "unwatch", "createDatabase",
    "joinSession", "recordGuide", "recordCommandGuide", "replayGuide", "embed", "applyTag", "tagTable", "referenceBlock",
    "createCanvas", "importCanvas", "exportCanvas",
];

fn id() -> fs_types::FieldDescription {
    reference("id", "Page", "wiki.page.id")
}

/// Add the wished commands.
pub(crate) fn add(d: TypeDescription) -> TypeDescription {
    sharing(collaboration(knowledge(d)))
}

/// Permissions and sharing (Sharing interface, open); watch and notifications (`subscribe` in the hub).
fn sharing(d: TypeDescription) -> TypeDescription {
    d.command(
        command(T, "share", K::Other)
            .input(id())
            .input(field(T, "with", B::Text).required())
            .input(field(T, "right", B::Choice).choices(&["read", "comment", "edit"]).required()),
    )
    .command(command(T, "unshare", K::Other).input(id()).input(field(T, "with", B::Text).required()))
    .command(command(T, "watch", K::Other).input(id()))
    .command(command(T, "unwatch", K::Other).input(id()))
}

/// Comments, talk pages (messages with a reference to the page); attachments (files with a reference); labels.
fn collaboration(d: TypeDescription) -> TypeDescription {
    d.command(command(T, "comment", K::Create).input(id()).input(field(T, "text", B::MultilineText).required()))
        .command(command(T, "openTalkPage", K::Other).input(id()).result(optional_reference("talk", "Page", "wiki.page.talk")))
        .command(command(T, "attach", K::Other).input(id()).input(reference("file", "File", "wiki.page.file")))
        .command(command(T, "detach", K::Other).input(id()).input(reference("file", "File", "wiki.page.file")))
        .command(command(T, "setLabels", K::Change).input(id()).input(field(T, "labels", B::Keywords)))
        .command(command(T, "joinSession", K::Other).input(id()))
}

/// Optional ideas of `requirements.md`: databases, guides, embeds, typed notes, block references, canvas.
fn knowledge(d: TypeDescription) -> TypeDescription {
    d.command(command(T, "createDatabase", K::Create).input(field(T, "path", B::Text).required()).input(field(T, "columns", B::Keywords)))
        .command(command(T, "recordGuide", K::Create).input(field(T, "path", B::Text).required()).input(field(T, "title", B::Text)))
        .command(command(T, "recordCommandGuide", K::Create).input(field(T, "path", B::Text).required()).input(field(T, "title", B::Text)))
        .command(command(T, "replayGuide", K::Other).input(id()))
        .command(command(T, "embed", K::Other).input(id()).input(field(T, "block", B::Text)).result(field(T, "content", B::MultilineText)))
        .command(command(T, "applyTag", K::Change).input(id()).input(field(T, "tag", B::Text).required()))
        .command(command(T, "tagTable", K::Find).input(field(T, "tag", B::Text).required()))
        .command(command(T, "referenceBlock", K::Other).input(id()).input(field(T, "block", B::Text).required()).result(field(T, "link", B::Link)))
        .command(command(T, "createCanvas", K::Create).input(field(T, "path", B::Text).required()))
        .command(command(T, "importCanvas", K::Create).input(field(T, "path", B::Text).required()).input(field(T, "text", B::MultilineText).required()))
        .command(command(T, "exportCanvas", K::Other).input(id()).result(field(T, "text", B::MultilineText)))
}
