//! Drafts whose page is gone from their path.
//!
//! Pattern: Read model over the draft folder — a draft is stored under the path of its page; after
//! a move or a delete nothing asks for that path again, so the caller lists these.

use fs_components_workbenches::{Draft, SourceError};

use crate::drafts::{FileDraftStore, Stored, failed};

impl FileDraftStore {
    /// The drafts whose page `exists` no longer knows (moved or deleted since): they are never
    /// offered again by path, so the caller shows them somewhere.
    ///
    /// # Errors
    /// A message when the folder or a backup cannot be read.
    pub fn orphans(&self, exists: &dyn Fn(&str) -> bool) -> Result<Vec<Draft>, SourceError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(failed(&e)),
        };
        let mut found = vec![];
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "draft") {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| failed(&e))?;
            let s: Stored = serde_json::from_slice(&bytes).map_err(|e| {
                SourceError::failed(format!("the backup {} is damaged: {e}", path.display()))
            })?;
            if !exists(&s.path) {
                found.push(Draft::new(s.path, s.text, s.original, s.revision));
            }
        }
        found.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(found)
    }
}
