//! Ticking a checkbox on the wiki: one ordinary change of the stored text.
//!
//! Pattern: Command — the operations of `task_ops` are applied to the document of the entry and stored like a saved edit
//! (`Texts::edit_if`: a conflict is refused, the change appears in the history and can be undone there).

use fs_components_workbenches::SourceError;

use super::wiki::{WikiSource, failed, revision_of};
use crate::task_ops::tick_ops;

impl WikiSource {
    /// Set box number `index` of the page at `path` to `done` (see `task_ops`). `revision` is the one of the page shown.
    ///
    /// # Errors
    /// [`SourceError`]: a conflict (someone saved meanwhile), a box that is not there, or a store that refused.
    pub fn tick(
        &self,
        path: &str,
        index: usize,
        done: bool,
        revision: Option<&str>,
    ) -> Result<(), SourceError> {
        let entry = self.entry(&self.text_for(path))?;
        if revision.is_some_and(|r| r != revision_of(&entry)) {
            return Err(SourceError::conflict(
                "Someone else changed this page since you opened it. Open it again and tick the box then.",
            ));
        }
        let ops = tick_ops(&entry.document, index, done).map_err(|e| {
            SourceError::unsupported(format!("there is no checkbox number {}", e.0))
        })?;
        if ops.is_empty() {
            return Ok(());
        }
        let mut document = entry.document.clone();
        for op in &ops {
            document.apply(op).map_err(|e| failed(&e))?;
        }
        match self
            .runtime
            .block_on(self.texts.edit_if(entry.id, entry.revision, document))
        {
            Ok(_) => {
                self.saved();
                Ok(())
            }
            Err(fs_wiki_core::Error::Conflict { .. }) => Err(SourceError::conflict(
                "Someone else changed this page since you opened it. Open it again and tick the box then.",
            )),
            Err(e) => Err(failed(&e)),
        }
    }
}
