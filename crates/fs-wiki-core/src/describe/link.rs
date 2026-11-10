//! `Link`: a link from a page to another page or an outside address (`LinkEntry`).
//!
//! Pattern: Value object. Standard: W3C `ActivityStreams` 2.0 `Link` (target, text of the link). The list is read-only: links
//! are made by writing the page. The id is `<source>|<target>|<place>` (derived, not stored).

use fs_types::{BasicType as B, CommandKind as K, DescribesType, TypeDescription};

use super::{command, field, optional_reference, reference, PROGRAM};
use crate::LinkEntry;

const T: &str = "link";

/// Wished commands of `Link` (`requirements.md`: unlinked mentions).
pub(crate) const WISHED: &[&str] = &["unlinkedMentions"];

fn describe() -> TypeDescription {
    TypeDescription::new("Link", PROGRAM, "W3C ActivityStreams 2.0 Link", "wiki.link")
        .field(reference("id", "Link", "wiki.link.id"))
        .field(reference("source", "Page", "wiki.link.source"))
        .field(field(T, "target", B::Text).required())
        .field(field(T, "at", B::Text))
        .field(field(T, "title", B::Text))
        .field(field(T, "internal", B::Choice).choices(&["yes", "no"]))
        .command(
            command(T, "find", K::Find)
                .input(optional_reference("source", "Page", "wiki.link.source"))
                .input(field(T, "target", B::Text)),
        )
        .command(command(T, "unlinkedMentions", K::Find).input(reference(
            "page",
            "Page",
            "wiki.link.page",
        )))
}

impl DescribesType for LinkEntry {
    fn describe() -> TypeDescription {
        describe()
    }
}
