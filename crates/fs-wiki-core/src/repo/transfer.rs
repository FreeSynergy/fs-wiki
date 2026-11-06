//! Import and export by media type.
//!
//! Pattern: Template method — the file is never the truth (ADR 0042): `import_text` on a new path
//! creates a text; on an existing path it is an outside change, so the stored document is
//! compared with the file and only the difference is applied as ordinary edit ops (layers move
//! with the text; text that is gone takes its marks along). An unchanged file writes nothing.

use fs_store::ObjectId;

use super::handle::WikiRepo;
use crate::error::{Error, Result};
use crate::media::TextFormat;
use crate::path::TextPath;
use crate::texts::TextEntry;

impl WikiRepo {
    pub(super) async fn import_locked(
        &self,
        path: &TextPath,
        format: TextFormat,
        text: &str,
    ) -> Result<TextEntry> {
        let Some(existing) = self.text_at(path.as_str()).await? else {
            return self.create_locked(path, format.import(text)).await;
        };
        let mut doc = existing.document.clone();
        if !format.import_external(&mut doc, text) {
            return Ok(existing);
        }
        self.edit_locked(existing.id, doc).await
    }

    pub(super) async fn export_of(&self, id: ObjectId, format: TextFormat) -> Result<String> {
        let text = self.text(id).await?.ok_or(Error::NotFound(id))?;
        Ok(format.export(&text.document))
    }
}
