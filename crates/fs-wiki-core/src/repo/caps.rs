//! The capability report of the store-backed provider.
//!
//! Pattern: none (one function) — every id of the schema, once.

use fs_interfaces::Capabilities;

/// The store-backed provider offers every method of the contract. Without the purge port
/// (`with_log_purge`) it cannot remove an object from the log, so `text.trash` and `text.history`
/// (whose `purge` and `purgeRevision` need that) are not offered.
pub(super) fn capabilities(can_purge: bool) -> Capabilities {
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
        if !can_purge && (id == "text.trash" || id == "text.history") {
            caps.unsupported(id, "this backend cannot remove an object from its log");
        } else {
            caps.supported(id);
        }
    }
    caps
}
