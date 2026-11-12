//! The heads of all texts (id, path, title, revision), and a file to start from.
//!
//! Pattern: Cache-aside Repository — the page tree and the linking of language variants need only these four things per text,
//! but `Texts::list` brings every document with them, which for the whole documentation (4199 texts) takes minutes on the
//! server. With `--index FILE` the heads are read from that file when it is there (the window opens at once); when it is not,
//! they are listed once and written. A text changed by another program after that is seen after the file is removed (the
//! reading wiki only reads; a refresh in the background is a task of the server, see DESIGN).

use std::path::Path;

use fs_objects::ObjectId;
use fs_wiki_core::{Paging, Texts};
use serde::{Deserialize, Serialize};

/// What the tree needs of one text.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Head {
    /// The text.
    pub id: ObjectId,
    /// Its path.
    pub path: String,
    /// Its name (the title).
    pub title: String,
    /// Its revision.
    pub revision: u64,
}

/// The heads of every text: from `cache` if the file is there, else listed (and written to `cache`).
///
/// # Errors
/// A message: the list failed, or the file is unreadable.
pub async fn load_or_list(texts: &dyn Texts, cache: Option<&Path>) -> Result<Vec<Head>, String> {
    if let Some(file) = cache.filter(|f| f.is_file()) {
        let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        return serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()));
    }
    let list = texts
        .list("", Paging::default())
        .await
        .map_err(|e| e.to_string())?;
    let heads: Vec<Head> = list
        .into_iter()
        .map(|t| Head {
            id: t.id,
            path: t.path,
            title: t.envelope.name,
            revision: t.revision,
        })
        .collect();
    if let Some(file) = cache {
        let temp = file.with_extension("tmp");
        let written = serde_json::to_vec(&heads)
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(&temp, b).map_err(|e| e.to_string()))
            .and_then(|()| std::fs::rename(&temp, file).map_err(|e| e.to_string()));
        if let Err(e) = written {
            eprintln!("fs-wiki: the index is not saved ({}: {e})", file.display());
        }
    }
    Ok(heads)
}
