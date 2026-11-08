//! Moving a text: a new path, and every link to the old one follows.
//!
//! Pattern: Command producer — `relinked` (relink.rs) turns the move into ops on the other texts'
//! documents; this file stores the results. The other texts are written first and the moved text
//! last, so a move that stops half way is finished by repeating it: texts already rewritten no
//! longer link to the old path. (`fs-objects` 0.2 has no transaction over several objects.) The
//! links of the moved text itself are written again for its new place.

use fs_objects::{ObjectId, Query};
use fs_text::Document;

use super::handle::WikiRepo;
use super::read::all_texts;
use super::record::entry_of;
use super::relink::relinked;
use super::revise::How;
use super::shift::Shift;
use crate::error::{Error, Result};
use crate::feed::TextChange;
use crate::path::TextPath;
use crate::texts::{Paging, TextEntry};

impl WikiRepo {
    pub(super) async fn move_locked(&self, id: ObjectId, new: &TextPath) -> Result<TextEntry> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        if text.path == new.as_str() {
            return Ok(text);
        }
        self.ensure_free(new, Some(id)).await?;
        let paths = self.all_paths().await?;
        let shift = Shift {
            old: &text.path,
            new: new.as_str(),
            folder: false,
            paths: &paths,
        };

        // Texts that link to the old path, and texts whose links could now lead to the new one.
        let linking: Query = all_texts(Paging::default()).with_index("link", &text.path);
        let arriving: Query = all_texts(Paging::default()).with_index("link", new.as_str());
        let mut seen = std::collections::HashSet::new();
        for query in [linking, arriving] {
            for other in self.reads.query(&query).await? {
                if other.envelope.id == id || !seen.insert(other.envelope.id) {
                    continue;
                }
                let entry = entry_of(other)?;
                if let Some(doc) = relinked(&entry.document, &entry.path, &entry.path, &shift) {
                    self.store_document(&entry, &entry.path, &doc).await?;
                }
            }
        }

        let doc = relinked(&text.document, &text.path, new.as_str(), &shift)
            .unwrap_or_else(|| text.document.clone());
        self.store_document(&text, new.as_str(), &doc).await?;
        self.entry(id).await
    }

    /// Write `doc` as the next revision of `prior`, at `path`.
    pub(super) async fn store_document(
        &self,
        prior: &TextEntry,
        path: &str,
        doc: &Document,
    ) -> Result<()> {
        let summary = if path == prior.path {
            "links updated"
        } else {
            "moved"
        };
        self.write_text(prior.id, Some(prior), path, doc, How::said(summary))
            .await?;
        self.feed.notify(&TextChange::Updated(prior.id));
        Ok(())
    }
}
