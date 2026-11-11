//! `Page` for the Admin frontend: the list (`find`, `search`) and the commands.
//!
//! Pattern: Strategy by command — `run` is one `match` on the command name of the description; each arm is one call of
//! the core (`Texts`). A command that is described but not carried out answers `unsupported:<name>` (the table is in
//! `fs-wiki-core`, `describe::is_supported`). Changing is partial (the rule of `Backend::run`): a field not sent stays.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_objects::ObjectId;
use fs_types::Edit;
use fs_wiki_core::describe::is_supported;
use fs_wiki_core::{Expect, Paging, SearchFilter, SearchQuery, TextFormat, Tri};

use super::ctx::{
    Ctx, LIST_LIMIT, id, millis, need, one, optional_id, placement, sent, unsupported,
};
use super::{page_more, records};

/// The pages of the wiki.
///
/// It remembers the revision of every page it listed: a `change` is written against the revision the person saw, so a
/// write in between is a conflict, not lost (the typed `Edit` carries no revision).
pub struct PageBackend {
    ctx: Ctx,
    seen: Mutex<HashMap<ObjectId, u64>>,
}

fn tri(input: &Values, field: &str) -> Tri {
    match one(input, field) {
        Some("only") => Tri::Only,
        Some("exclude") => Tri::Exclude,
        _ => Tri::Any,
    }
}

impl PageBackend {
    /// The pages of the wiki, asked through `ctx`.
    pub fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// Remember what a list showed: the revision of each page.
    fn remember(&self, records: &[Record]) {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        for r in records {
            let revision = r.values.get("revision").and_then(|v| v.first());
            if let (Ok(id), Some(Ok(n))) = (r.id.parse(), revision.map(|s| s.parse::<u64>())) {
                seen.insert(id, n);
            }
        }
    }

    fn search(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        let query = SearchQuery {
            term: one(q, "term").unwrap_or_default().to_owned(),
            prefix: one(q, "prefix") == Some("yes"),
            filter: SearchFilter {
                path_prefix: one(q, "pathPrefix").unwrap_or_default().to_owned(),
                template: tri(q, "templates"),
                redirect: tri(q, "redirects"),
                author: one(q, "author").unwrap_or_default().to_owned(),
                space: optional_id(q, "space")?,
                updated_from_ms: millis(q, "updatedFrom")?,
                updated_to_ms: millis(q, "updatedTo")?,
            },
        };
        let hits = self
            .ctx
            .block(self.ctx.texts.search(&query, None, LIST_LIMIT))?
            .items;
        // A hit carries no text: read each page, so the detail shows every field.
        let mut out = Vec::new();
        for h in hits {
            match self.ctx.block(self.ctx.texts.get(h.id))? {
                Some(e) => out.push(records::page(&e)),
                None => out.push(records::hit(&h)),
            }
        }
        Ok(out)
    }

    /// The typed edit of a page (`Backend::change`): the fields that are in the edit are written, the others stay. The
    /// content is read as an outside change, so layers stay. The page is read first and the content is written against the
    /// revision just read, so a write in between is a conflict, not lost. The steps run one after the other (content,
    /// kind, redirect, path); a refusal stops at the step that was refused and the earlier ones stay.
    fn change(&self, record: &str, edit: &Edit) -> Result<Outcome, BackendError> {
        let page: ObjectId = record.parse().map_err(|_| BackendError::NotFound)?;
        let current = self
            .ctx
            .block(self.ctx.texts.get(page))?
            .ok_or(BackendError::NotFound)?;
        let t = &self.ctx.texts;
        let seen = self
            .seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&page)
            .copied();
        let value = |name: &str| {
            edit.fields
                .iter()
                .find(|f| f.path.len() == 1 && f.path[0] == name)
                .map(|f| f.values.as_slice())
        };
        if let Some(values) = value("content") {
            let mut doc = current.document.clone();
            TextFormat::Markdown
                .import_external(&mut doc, values.first().map_or("", String::as_str));
            let written = self.ctx.block(t.edit_with(
                page,
                Expect::Revision(seen.unwrap_or(current.revision)),
                doc,
                "",
            ))?;
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(page, written.revision);
        }
        if let Some(values) = value("kind") {
            let template = values.first().is_some_and(|k| k == "template");
            self.ctx
                .block(t.set_template(page, template, Expect::Any))?;
        }
        if let Some(values) = value("redirectTo") {
            // No value clears the redirect.
            let target = values.first().map_or("", |v| v.trim());
            self.ctx.block(t.set_redirect(page, target, Expect::Any))?;
        }
        if let Some(path) = value("path").and_then(<[String]>::first)
            && *path != current.path
        {
            self.ctx.block(t.move_to(page, path.trim()))?;
        }
        Ok(records::outcome(page))
    }

    /// The records the command `find` selects (`list` remembers their revisions).
    fn find(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        if q.contains_key("tag") {
            return Err(unsupported("tagTable"));
        }
        let search_only = [
            "prefix",
            "pathPrefix",
            "templates",
            "redirects",
            "author",
            "space",
            "updatedFrom",
            "updatedTo",
        ];
        if search_only
            .iter()
            .any(|k| one(q, k).is_some_and(|v| v != "any" && v != "no"))
        {
            return self.search(q);
        }
        let term = one(q, "term").unwrap_or_default();
        let paging = Paging {
            offset: 0,
            limit: Some(LIST_LIMIT),
        };
        let texts = if term.is_empty() {
            self.ctx.block(self.ctx.texts.list("", paging))?
        } else {
            self.ctx.block(self.ctx.texts.find(term, paging))?
        };
        Ok(texts.iter().map(records::page).collect())
    }
}

impl Backend for PageBackend {
    fn supported(&self, command: &str) -> bool {
        is_supported("Page", command)
    }

    fn change(&self, record: &str, edit: &Edit) -> Result<Outcome, BackendError> {
        PageBackend::change(self, record, edit)
    }

    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
        let records = self.find(q)?;
        self.remember(&records);
        Ok(records)
    }

    fn run(
        &self,
        name: &str,
        _record: Option<&str>,
        input: &Values,
    ) -> Result<Outcome, BackendError> {
        if !is_supported("Page", name) {
            return Err(unsupported(name));
        }
        let t = &self.ctx.texts;
        match name {
            "create" => {
                let e = self.ctx.block(t.create(
                    need(input, "path")?,
                    one(input, "content").unwrap_or_default(),
                ))?;
                Ok(records::outcome(e.id))
            }
            "import" => {
                let e = self.ctx.block(t.import_text(
                    need(input, "path")?,
                    need(input, "mediaType")?,
                    need(input, "text")?,
                ))?;
                Ok(records::outcome(e.id))
            }
            "createFromTemplate" => {
                let e = self
                    .ctx
                    .block(t.create_from_template(id(input, "template")?, need(input, "path")?))?;
                Ok(records::outcome(e.id))
            }
            "move" => {
                let e = self
                    .ctx
                    .block(t.move_to(id(input, "id")?, need(input, "path")?))?;
                Ok(records::outcome(e.id))
            }
            "setTemplate" => {
                let on = need(input, "kind")? == "template";
                let e = self
                    .ctx
                    .block(t.set_template(id(input, "id")?, on, Expect::Any))?;
                Ok(records::outcome(e.id))
            }
            "setRedirect" => {
                let page = id(input, "id")?;
                // A field not sent is unchanged; an empty list clears the redirect.
                if !sent(input, "redirectTo") {
                    return Ok(records::outcome(page));
                }
                let target = one(input, "redirectTo").unwrap_or_default();
                let e = self.ctx.block(t.set_redirect(page, target, Expect::Any))?;
                Ok(records::outcome(e.id))
            }
            "reorder" => {
                let at = placement(need(input, "placement")?)?;
                let n = self.ctx.block(t.reorder(
                    id(input, "id")?,
                    at,
                    optional_id(input, "anchor")?,
                ))?;
                Ok(records::outcome(n.id))
            }
            "trash" => {
                let item = self.ctx.block(t.trash(id(input, "id")?))?;
                Ok(records::outcome(item.id))
            }
            "delete" => {
                self.ctx.block(t.delete(id(input, "id")?))?;
                Ok(Outcome::default())
            }
            _ => page_more::run(&self.ctx, name, input),
        }
    }
}
