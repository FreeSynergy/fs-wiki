//! Moving a text: a new path, and every link to the old one follows.
//!
//! Pattern: Command producer — `relinked` (relink.rs) turns the move into ops on the other texts'
//! documents; this file stores the results. The other texts are written first and the moved text
//! last, so a move that stops half way is finished by repeating it: texts already rewritten no
//! longer link to the old path. (`fs-store` 0.2 has no transaction over several objects.) The
//! links of the moved text itself are written again for its new place.

use fs_store::{ObjectId, Query};
use fs_text::Document;

use super::handle::WikiRepo;
use super::read::all_texts;
use super::record::entry_of;
use super::relink::relinked;
use super::shift::Shift;
use crate::error::{Error, Result};
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

        let linking: Query = all_texts(Paging::default()).with_index("link", &text.path);
        for other in self.reads.query(&linking).await? {
            if other.envelope.id == id {
                continue;
            }
            let entry = entry_of(other)?;
            if let Some(doc) = relinked(&entry.document, &entry.path, &entry.path, &shift) {
                self.store_document(&entry, &entry.path, &doc).await?;
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
        let obj = self.build_object(prior.id, Some(prior), path, doc)?;
        self.objects.put(obj, &self.actor).await?;
        Ok(())
    }
}
