//! `Revision` for the Admin frontend: the history of a page.
//!
//! Pattern: Strategy by command, as `page.rs`. A revision's id is `<page id>#<number>`; `find` needs the page.

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_objects::ObjectId;
use fs_wiki_core::describe::is_supported;
use fs_wiki_core::{Expect, TextFormat};

use super::ctx::{need, not_a_number, one, unsupported, Ctx, LIST_LIMIT};
use super::records;

/// The revisions of the pages.
pub struct RevisionBackend(pub Ctx);

fn split(input: &Values) -> Result<(ObjectId, u64), BackendError> {
    let (page, number) = need(input, "id")?.split_once('#').ok_or(BackendError::NotFound)?;
    Ok((page.parse().map_err(|_| BackendError::NotFound)?, number.parse().map_err(|_| not_a_number("id"))?))
}

impl Backend for RevisionBackend {
    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        // the history of one page; without a page there is nothing to list (the history is per text)
        let Some(page) = one(q, "page") else { return Ok(Vec::new()) };
        let page: ObjectId = page.parse().map_err(|_| BackendError::NotFound)?;
        let h = self.0.block(self.0.texts.history(page, None, LIST_LIMIT))?;
        Ok(h.items.iter().map(records::revision).collect())
    }

    fn run(&self, name: &str, input: &Values) -> Result<Outcome, BackendError> {
        if !is_supported("Revision", name) {
            return Err(unsupported(name));
        }
        let t = &self.0.texts;
        let (page, number) = split(input)?;
        match name {
            "get" => {
                let r = self.0.block(t.get_revision(page, number))?.ok_or(BackendError::NotFound)?;
                let mut values = Values::new();
                values.insert("content".into(), vec![TextFormat::Markdown.export(&r.document)]);
                Ok(Outcome { id: None, values })
            }
            "restore" => {
                let expect = match one(input, "revision").map(str::parse::<u64>) {
                    Some(Ok(n)) => Expect::Revision(n),
                    Some(Err(_)) => return Err(not_a_number("revision")),
                    None => Expect::Any,
                };
                let e = self.0.block(t.restore_revision(page, number, expect))?;
                Ok(records::outcome(e.id))
            }
            "purge" => {
                self.0.block(t.purge_revision(page, number))?;
                Ok(Outcome::default())
            }
            other => Err(unsupported(other)),
        }
    }
}
