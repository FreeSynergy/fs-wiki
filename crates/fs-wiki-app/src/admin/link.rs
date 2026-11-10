//! `Link` for the Admin frontend: links out of a page and links into a path.
//!
//! Pattern: Strategy by input — `find` with a `source` page lists its links, with a `target` path lists the links into
//! it (`backlinks`); with neither there is nothing to list (the link table is per page). Read-only.

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};

use super::ctx::{optional_id, unsupported, Ctx};
use super::records;

/// The link table.
pub struct LinkBackend(pub Ctx);

impl Backend for LinkBackend {
    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        if q.contains_key("page") {
            return Err(unsupported("unlinkedMentions"));
        }
        let rows = if let Some(source) = optional_id(q, "source")? {
            self.0.block(self.0.texts.links(source))?
        } else if let Some(target) = super::ctx::one(q, "target") {
            self.0.block(self.0.texts.backlinks(target))?
        } else {
            Vec::new()
        };
        Ok(rows.iter().map(records::link).collect())
    }

    fn run(&self, name: &str, _: &Values) -> Result<Outcome, BackendError> {
        // Links are made by writing a page; nothing here is carried out.
        Err(unsupported(name))
    }
}
