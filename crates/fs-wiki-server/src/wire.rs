//! Texts on the wire.
//!
//! Pattern: Adapter — `TextEntry`, pages and request values between Rust and `texts.capnp`. The JSON
//! form of the document stays authoritative for the content; the envelope is the routing summary.
//! Every size is checked here, so that no answer can end the connection and no request can store
//! what could not come back.

use fs_interfaces::rpc::{
    check_answer, check_intake, envelope_weight, too_large_by, write_envelope, MAX_INTAKE_BYTES,
    MAX_PAGE_BYTES,
};
use fs_interfaces::texts_capnp::text_entry;
use fs_text::Document;
use fs_wiki_core::TextEntry;

use crate::errors::invalid;

/// A text ready to be written: its entry and its document as JSON, made once.
pub(crate) struct Sent {
    entry: TextEntry,
    json: String,
}

impl Sent {
    pub(crate) fn new(entry: TextEntry) -> capnp::Result<Self> {
        let json = serde_json::to_string(&entry.document)
            .map_err(|e| capnp::Error::failed(format!("document does not write: {e}")))?;
        Ok(Self { entry, json })
    }

    /// What this text weighs in an answer.
    pub(crate) fn weight(&self) -> usize {
        self.json.len() + self.entry.path.len() + envelope_weight(&self.entry.envelope)
    }
}

pub(crate) fn count(n: usize) -> capnp::Result<u32> {
    u32::try_from(n).map_err(|_| capnp::Error::failed("list too long".into()))
}

/// Write one text. The answer must fit a message.
pub(crate) fn write_entry(mut out: text_entry::Builder<'_>, sent: &Sent) -> capnp::Result<()> {
    check_answer(sent.weight())?;
    fill(&mut out, sent);
    Ok(())
}

pub(crate) fn fill(out: &mut text_entry::Builder<'_>, sent: &Sent) {
    write_envelope(out.reborrow().init_envelope(), &sent.entry.envelope);
    out.set_path(sent.entry.path.as_str());
    out.set_document(sent.json.as_str());
    // Only equality means anything: `editIf` hands it back unchanged. It is no counter and no time.
    out.set_revision(sent.entry.revision);
}

/// Write the texts of one answer without a cursor (`moveFolder`): all, or a refusal that says what
/// was done. The write has happened by now, so the refusal names how to read the result.
pub(crate) fn write_all(
    mut out: capnp::struct_list::Builder<'_, text_entry::Owned>,
    texts: Vec<TextEntry>,
) -> capnp::Result<()> {
    let sent: Vec<Sent> = texts.into_iter().map(Sent::new).collect::<Result<_, _>>()?;
    let total: usize = sent.iter().map(Sent::weight).sum();
    if total > MAX_PAGE_BYTES {
        return Err(too_large_by(
            format!(
                "the {} moved texts weigh {total} bytes, at most {MAX_PAGE_BYTES} fit; the move is done, read them with list",
                sent.len()
            ),
            total as u64,
            MAX_PAGE_BYTES as u64,
        ));
    }
    for (i, s) in (0u32..).zip(&sent) {
        fill(&mut out.reborrow().get(i), s);
    }
    Ok(())
}

/// A text of a request (a path, a term, Markdown): bounded, may be empty.
pub(crate) fn text_arg(text: capnp::text::Reader<'_>) -> capnp::Result<&str> {
    let text = text.to_str()?;
    check_intake(text.len())?;
    Ok(text)
}

/// Largest Markdown (or other source text) a request may carry. Smaller than the intake limit of a
/// document: the server parses it on the thread that serves every connection, and a document is far
/// larger than its Markdown (about 100 times for one-letter paragraphs).
pub const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;

/// The source text of a request (Markdown for `create`, the file for `importText`): bounded.
pub(crate) fn source_arg(text: capnp::text::Reader<'_>) -> capnp::Result<&str> {
    let text = text.to_str()?;
    if text.len() > MAX_SOURCE_BYTES {
        return Err(too_large_by(
            format!(
                "{} bytes of text, at most {MAX_SOURCE_BYTES} are taken",
                text.len()
            ),
            text.len() as u64,
            MAX_SOURCE_BYTES as u64,
        ));
    }
    Ok(text)
}

/// A text of a request that must not be empty.
pub(crate) fn required<'a>(text: capnp::text::Reader<'a>, what: &str) -> capnp::Result<&'a str> {
    let text = text_arg(text)?;
    if text.is_empty() {
        return Err(invalid(format!("{what} must not be empty"), &format!("{what}-empty")));
    }
    Ok(text)
}

/// The document of a request, from its JSON form.
pub(crate) fn read_document(text: capnp::text::Reader<'_>) -> capnp::Result<Document> {
    let text = text_arg(text)?;
    serde_json::from_str(text).map_err(|e| invalid(format!("the document does not read: {e}"), "document-unreadable"))
}

/// Refuse a document whose JSON form could not come back in an answer, before it is stored. Used for
/// Markdown, whose document may be many times larger than the text it came from.
pub(crate) fn fits(document: &Document) -> capnp::Result<()> {
    let json = serde_json::to_string(document)
        .map_err(|e| capnp::Error::failed(format!("document does not write: {e}")))?;
    if json.len() > MAX_INTAKE_BYTES {
        return Err(too_large_by(
            format!(
                "the text becomes {} bytes as a document, at most {MAX_INTAKE_BYTES} fit",
                json.len()
            ),
            json.len() as u64,
            MAX_INTAKE_BYTES as u64,
        ));
    }
    Ok(())
}
