//! Moving a text: a new path, and every link to the old one follows.
//!
//! Pattern: Command producer + Strategy — `rewrite_links_with` of fs-text turns the move into
//! ops on the other texts' documents (the Strategy is [`MovePage`]); this file stores the
//! results. The other texts are written first and the moved text last, so a move that stops
//! half way is finished by repeating it: texts already rewritten no longer link to the old
//! path. (`fs-objects` 0.2 has no transaction over several objects.)

use fs_objects::{ObjectId, Query};
use fs_text::{moved_path, rewrite_links_with, Document, LinkRewriter};

use super::handle::WikiRepo;
use super::read::all_texts;
use crate::error::{Error, Result};
use crate::path::{page_of, TextPath};
use crate::texts::{Paging, TextEntry};

/// Links to exactly the page `old` (a `#fragment` is kept) now point to `new`. Links to
/// `old/...` are other pages and stay.
struct MovePage<'a> {
    old: &'a str,
    new: &'a str,
}

impl LinkRewriter for MovePage<'_> {
    fn rewrite(&self, path: &str) -> Option<String> {
        if page_of(path) == self.old {
            moved_path(path, self.old, self.new)
        } else {
            None
        }
    }
}

impl WikiRepo {
    pub(super) async fn move_locked(&self, id: ObjectId, new: &TextPath) -> Result<TextEntry> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        if text.path == new.as_str() {
            return Ok(text);
        }
        self.ensure_free(new, Some(id)).await?;
        let rewriter = MovePage {
            old: &text.path,
            new: new.as_str(),
        };

        let linking: Query = all_texts(Paging::default()).with_index("link", &text.path);
        for other in self.reads.query(&linking).await? {
            let other_id = other.envelope.id;
            if other_id == id {
                continue;
            }
            let entry = super::record::entry_of(other)?;
            let mut docs = [entry.document];
            if !rewrite_links_with(&mut docs, &rewriter).is_empty() {
                let [doc] = docs;
                self.store_document(&entry.envelope, &entry.path, &doc)
                    .await?;
            }
        }

        let mut docs = [text.document];
        rewrite_links_with(&mut docs, &rewriter);
        let [doc]: [Document; 1] = docs;
        self.store_document(&text.envelope, new.as_str(), &doc)
            .await?;
        self.entry(id).await
    }

    pub(super) async fn store_document(
        &self,
        envelope: &fs_objects::Envelope,
        path: &str,
        doc: &Document,
    ) -> Result<()> {
        let obj = self.build_object(envelope.id, Some(envelope), path, doc)?;
        self.objects.put(obj, &self.actor).await?;
        Ok(())
    }
}
