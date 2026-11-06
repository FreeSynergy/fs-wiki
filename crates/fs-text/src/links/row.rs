//! The link table: one row per link / image mark.
//!
//! Pattern: Iterator over marks -> read model (table rows); back references read the same
//! table.

use crate::doc::Document;
use crate::id::Uuid;
use crate::mark::MarkKind;
use serde::{Deserialize, Serialize};

/// One row of the link table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkRow {
    /// Document containing the link.
    pub source_doc: Uuid,
    /// Block containing the link.
    pub source_block: Uuid,
    /// The link mark.
    pub mark: Uuid,
    /// Char range of the link text in the source block (start).
    pub start: usize,
    /// End of the link text.
    pub end: usize,
    /// Readable path / URL.
    pub path: String,
    /// Resolved target (document / block), if known.
    pub target: Option<Uuid>,
    /// Link text as shown.
    pub title: String,
    /// Points to an internal page?
    pub internal: bool,
}

/// All link and image marks of `doc` as table rows.
#[must_use]
pub fn extract_links(doc: &Document) -> Vec<LinkRow> {
    let mut rows = Vec::new();
    for b in &doc.blocks {
        for m in b.marks.iter() {
            if !matches!(m.kind, MarkKind::Link | MarkKind::Image) {
                continue;
            }
            let Some(l) = m.link() else { continue };
            rows.push(LinkRow {
                source_doc: doc.id,
                source_block: b.id,
                mark: m.id,
                start: m.start,
                end: m.end,
                path: l.path.clone(),
                target: l.target,
                title: b.text.slice(m.start..m.end),
                internal: l.is_internal(),
            });
        }
    }
    rows
}

/// Rows pointing at `target` (back references come from the same table).
#[must_use]
pub fn backlinks(rows: &[LinkRow], target: Uuid) -> Vec<&LinkRow> {
    rows.iter().filter(|r| r.target == Some(target)).collect()
}
