//! `Page` for the Admin frontend: the list (`find`, `search`) and the commands.
//!
//! Pattern: Strategy by command — `run` is one `match` on the command name of the description; each arm is one call of
//! the core (`Texts`). A command that is described but not carried out answers `unsupported:<name>` (the table is in
//! `fs-wiki-core`, `describe::is_supported`). Changing is partial (the rule of `Backend::run`): a field not sent stays.

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_wiki_core::describe::is_supported;
use fs_wiki_core::{Expect, Paging, SearchFilter, SearchQuery, TextFormat, Tri};

use super::ctx::{
    Ctx, LIST_LIMIT, id, millis, need, not_a_number, one, optional_id, placement, sent, unsupported,
};
use super::{page_more, records};

/// The pages of the wiki.
pub struct PageBackend(pub Ctx);

fn tri(input: &Values, field: &str) -> Tri {
    match one(input, field) {
        Some("only") => Tri::Only,
        Some("exclude") => Tri::Exclude,
        _ => Tri::Any,
    }
}

impl PageBackend {
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
            .0
            .block(self.0.texts.search(&query, None, LIST_LIMIT))?
            .items;
        // A hit carries no text: read each page, so the detail shows every field.
        let mut out = Vec::new();
        for h in hits {
            match self.0.block(self.0.texts.get(h.id))? {
                Some(e) => out.push(records::page(&e)),
                None => out.push(records::hit(&h)),
            }
        }
        Ok(out)
    }

    fn change(&self, input: &Values) -> Result<Outcome, BackendError> {
        let page = id(input, "id")?;
        let current = self
            .0
            .block(self.0.texts.get(page))?
            .ok_or(BackendError::NotFound)?;
        let expect = match one(input, "revision").map(str::parse::<u64>) {
            Some(Ok(n)) => Expect::Revision(n),
            Some(Err(_)) => return Err(not_a_number("revision")),
            // No revision sent: the page as it was read just now, so a write in between is a conflict, not lost.
            None => Expect::Revision(current.revision),
        };
        let summary = one(input, "summary").unwrap_or_default().to_owned();
        let mut doc = current.document.clone();
        // A field not sent is unchanged; the Markdown is read as an outside change, so layers stay.
        if sent(input, "content") {
            let md = input
                .get("content")
                .and_then(|v| v.first())
                .map_or("", String::as_str);
            TextFormat::Markdown.import_external(&mut doc, md);
        }
        let written = self
            .0
            .block(self.0.texts.edit_with(page, expect, doc, &summary))?;
        Ok(records::outcome(written.id))
    }
}

impl Backend for PageBackend {
    fn list(&self, q: &Values) -> Result<Vec<Record>, BackendError> {
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
            self.0.block(self.0.texts.list("", paging))?
        } else {
            self.0.block(self.0.texts.find(term, paging))?
        };
        Ok(texts.iter().map(records::page).collect())
    }

    fn run(&self, name: &str, input: &Values) -> Result<Outcome, BackendError> {
        if !is_supported("Page", name) {
            return Err(unsupported(name));
        }
        let t = &self.0.texts;
        match name {
            "create" => {
                let e = self.0.block(t.create(
                    need(input, "path")?,
                    one(input, "content").unwrap_or_default(),
                ))?;
                Ok(records::outcome(e.id))
            }
            "import" => {
                let e = self.0.block(t.import_text(
                    need(input, "path")?,
                    need(input, "mediaType")?,
                    need(input, "text")?,
                ))?;
                Ok(records::outcome(e.id))
            }
            "createFromTemplate" => {
                let e = self
                    .0
                    .block(t.create_from_template(id(input, "template")?, need(input, "path")?))?;
                Ok(records::outcome(e.id))
            }
            "change" => self.change(input),
            "move" => {
                let e = self
                    .0
                    .block(t.move_to(id(input, "id")?, need(input, "path")?))?;
                Ok(records::outcome(e.id))
            }
            "setTemplate" => {
                let on = need(input, "kind")? == "template";
                let e = self
                    .0
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
                let e = self.0.block(t.set_redirect(page, target, Expect::Any))?;
                Ok(records::outcome(e.id))
            }
            "reorder" => {
                let at = placement(need(input, "placement")?)?;
                let n =
                    self.0
                        .block(t.reorder(id(input, "id")?, at, optional_id(input, "anchor")?))?;
                Ok(records::outcome(n.id))
            }
            "trash" => {
                let item = self.0.block(t.trash(id(input, "id")?))?;
                Ok(records::outcome(item.id))
            }
            "delete" => {
                self.0.block(t.delete(id(input, "id")?))?;
                Ok(Outcome::default())
            }
            _ => page_more::run(&self.0, name, input),
        }
    }
}
