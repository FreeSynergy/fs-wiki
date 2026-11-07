//! What the server sends, as the values of the core.
//!
//! Pattern: Adapter — `TextEntry` and `LinkRow` of `texts.capnp` to `TextEntry` and `LinkEntry`. A
//! reply that cannot be read is `Error::Store(Backend)`: the request is not to blame. `LinkRow` has no
//! title and no `internal` flag (`DESIGN.md`): the title is empty and `internal` is `true`, because the
//! server sends only links into the wiki.

use fs_interfaces::rpc::{read_envelope, read_object_id};
use fs_interfaces::texts_capnp::{link_row, text_entry};
use fs_wiki_core::{Error, LinkEntry, Result, TextEntry};

/// A reply that does not read.
pub(crate) fn unreadable(e: impl std::fmt::Display) -> Error {
    Error::Store(fs_objects::Error::Backend(format!(
        "wiki server: the reply does not read: {e}"
    )))
}

fn wire<T>(r: capnp::Result<T>) -> Result<T> {
    r.map_err(unreadable)
}

fn text(t: capnp::Result<capnp::text::Reader<'_>>) -> Result<String> {
    wire(t.and_then(|t| Ok(t.to_string()?)))
}

/// A text of the server: the envelope as sent, the document from its JSON form.
pub(crate) fn entry(r: text_entry::Reader<'_>) -> Result<TextEntry> {
    let envelope = wire(r.get_envelope().and_then(read_envelope))?;
    let document = serde_json::from_str(&text(r.get_document())?).map_err(unreadable)?;
    Ok(TextEntry {
        id: envelope.id,
        envelope,
        path: text(r.get_path())?,
        revision: r.get_revision(),
        document,
    })
}

/// A row of the link table. Only links into the wiki are sent, so every row is `internal`.
pub(crate) fn link(r: link_row::Reader<'_>) -> Result<LinkEntry> {
    Ok(LinkEntry {
        source: wire(r.get_source().and_then(read_object_id))?,
        target: text(r.get_target())?,
        at: text(r.get_at())?,
        title: String::new(),
        internal: true,
    })
}
