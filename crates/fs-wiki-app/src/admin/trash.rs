//! `TrashEntry` for the Admin frontend: the trash.
//!
//! Pattern: Strategy by command, as `page.rs`. `purge` follows a sync with the hub; without one the core says so and the
//! frontend shows it as `unavailable`.

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_wiki_core::describe::is_supported;

use super::ctx::{id, one, unsupported, Ctx, LIST_LIMIT};
use super::records;

/// The trashed pages.
pub struct TrashBackend(pub Ctx);

impl Backend for TrashBackend {
    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        let term = one(q, "term").unwrap_or_default().to_lowercase();
        let page = self.0.block(self.0.texts.list_trash(None, LIST_LIMIT))?;
        Ok(page
            .items
            .iter()
            .filter(|t| term.is_empty() || t.path.to_lowercase().contains(&term) || t.name.to_lowercase().contains(&term))
            .map(records::trash)
            .collect())
    }

    fn run(&self, name: &str, input: &Values) -> Result<Outcome, BackendError> {
        if !is_supported("TrashEntry", name) {
            return Err(unsupported(name));
        }
        let t = &self.0.texts;
        match name {
            "restore" => {
                let e = self.0.block(t.restore_from_trash(id(input, "id")?, one(input, "path").unwrap_or_default()))?;
                Ok(records::outcome(e.id))
            }
            "purge" => {
                self.0.block(t.purge(id(input, "id")?))?;
                Ok(Outcome::default())
            }
            other => Err(unsupported(other)),
        }
    }
}
