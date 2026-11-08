//! The capability report of the store-backed provider.
//!
//! Pattern: none (one function) — every id of the schema, once.

use fs_interfaces::Capabilities;

/// The store-backed provider offers every method of the contract.
pub(super) fn capabilities() -> Capabilities {
    let mut caps = Capabilities::new("fs-wiki");
    for id in [
        "text.create",
        "text.edit",
        "text.move",
        "text.delete",
        "text.read",
        "text.find",
        "link.read",
        "text.import",
        "text.export",
        "text.subscribe",
        "text.editSummary",
        "text.history",
        "text.trash",
        "text.space",
        "text.template",
        "text.redirect",
        "text.search",
    ] {
        caps.supported(id);
    }
    caps
}
