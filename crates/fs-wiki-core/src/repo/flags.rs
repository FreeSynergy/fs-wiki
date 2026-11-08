//! Templates and redirects: two flags of a text, and the copy made from a template.
//!
//! Pattern: Command — each is an ordinary write of the same document (a new revision with a summary
//! of its own), so history, `expect` and the feed work as for any edit. `resolve` is a read model:
//! it follows the redirects and reports how the chain ended, loop first.

use fs_objects::ObjectId;

use super::handle::WikiRepo;
use super::record::Flags;
use super::revise::How;
use crate::error::{Error, Result};
use crate::feed::TextChange;
use crate::history::Expect;
use crate::path::TextPath;
use crate::reason::Reason;
use crate::redirect::{Outcome, Resolution, MAX_REDIRECT_HOPS};
use crate::texts::TextEntry;

/// Whether `target` starts with a scheme (`https:`, `mailto:`): an outside address.
fn has_scheme(target: &str) -> bool {
    let mut chars = target.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && target.split(':').next().is_some_and(|s| {
            s.len() < target.len()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        })
}

impl WikiRepo {
    async fn rewrite_flags(
        &self,
        text: &TextEntry,
        flags: Flags,
        summary: &str,
    ) -> Result<TextEntry> {
        let how = How {
            summary,
            restored_from: 0,
            flags: Some(flags),
        };
        self.write_text(text.id, Some(text), &text.path, &text.document, how)
            .await?;
        self.feed.notify(&TextChange::Updated(text.id));
        self.entry(text.id).await
    }

    pub(super) async fn set_template_locked(
        &self,
        id: ObjectId,
        template: bool,
        expect: Expect,
    ) -> Result<TextEntry> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Self::check_expect(&text, expect)?;
        if text.template == template {
            return Ok(text);
        }
        let flags = Flags {
            template,
            redirect_to: text.redirect_to.clone(),
        };
        let summary = if template {
            "marked as template"
        } else {
            "template mark removed"
        };
        self.rewrite_flags(&text, flags, summary).await
    }

    pub(super) async fn set_redirect_locked(
        &self,
        id: ObjectId,
        target: &str,
        expect: Expect,
    ) -> Result<TextEntry> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Self::check_expect(&text, expect)?;
        if !target.is_empty() {
            if has_scheme(target) {
                return Err(Error::InvalidPath {
                    path: target.into(),
                    reason: Reason::ExternalTarget,
                });
            }
            TextPath::parse(target)?;
            if target == text.path {
                return Err(Error::Invalid {
                    issue: "self-redirect",
                    detail: "a text cannot redirect to itself".into(),
                });
            }
        }
        if text.redirect_to == target {
            return Ok(text);
        }
        let flags = Flags {
            template: text.template,
            redirect_to: target.into(),
        };
        let summary = if target.is_empty() {
            "redirect removed".to_string()
        } else {
            format!("redirect to {target}")
        };
        self.rewrite_flags(&text, flags, &summary).await
    }

    pub(super) async fn copy_template_locked(
        &self,
        template: ObjectId,
        path: &str,
    ) -> Result<TextEntry> {
        let source = self
            .text(template)
            .await?
            .ok_or(Error::NotFound(template))?;
        if !source.template {
            return Err(Error::Invalid {
                issue: "not-a-template",
                detail: format!("{} is not marked as a template", source.path),
            });
        }
        let path = TextPath::parse(path)?;
        self.ensure_free(&path, None).await?;
        let id = ObjectId::new();
        let mut doc = source.document.clone();
        doc.id = id.as_uuid();
        let summary = format!("from template {}", source.path);
        self.write_text(id, None, path.as_str(), &doc, How::said(&summary))
            .await?;
        self.feed.notify(&TextChange::Created(id));
        self.entry(id).await
    }

    pub(super) async fn resolve_path(&self, path: &str) -> Result<Resolution> {
        let mut hops: Vec<String> = vec![];
        let mut current = path.to_string();
        loop {
            let Some(text) = self.text_at(&current).await? else {
                hops.push(current);
                return Ok(Resolution {
                    outcome: Outcome::NotFound,
                    hops,
                    text: None,
                });
            };
            if text.redirect_to.is_empty() {
                return Ok(Resolution {
                    outcome: Outcome::Resolved,
                    hops,
                    text: Some(text),
                });
            }
            hops.push(current);
            current = text.redirect_to;
            if hops.contains(&current) {
                hops.push(current);
                return Ok(Resolution {
                    outcome: Outcome::Loop,
                    hops,
                    text: None,
                });
            }
            if hops.len() > MAX_REDIRECT_HOPS {
                return Ok(Resolution {
                    outcome: Outcome::TooManyHops,
                    hops,
                    text: None,
                });
            }
        }
    }
}
