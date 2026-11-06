//! The pages that did not import, as a page of their own.
//!
//! Pattern: Adapter — the list of failures becomes a document in the neutral model, so the
//! workbench shows it like any page and nothing is hidden.

use crate::report::Conflict;
use fs_components_widgets::{Block, Document, Inline, ListItem, Mark, MarkKind};

/// A file that could not be imported, with the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// Path of the file relative to the imported folder.
    pub path: String,
    /// Why it failed.
    pub reason: String,
}

/// The path of the problems page (no text can have this path: it contains `(`).
pub const PROBLEMS_PATH: &str = "(import problems)";

/// Everything the import found that needs a person: the failed files, the files that conflict
/// with their text, the files that are gone, the entries that were not looked at.
#[derive(Clone, Debug, Default)]
pub struct Notes {
    /// Files that failed to import.
    pub failures: Vec<Failure>,
    /// Files and texts that both changed.
    pub conflicts: Vec<Conflict>,
    /// Files that were imported once and are gone.
    pub gone: Vec<Failure>,
    /// Entries not looked at.
    pub skipped: Vec<Failure>,
}

impl Notes {
    /// How many things there are to look at.
    pub fn count(&self) -> usize {
        self.failures.len() + self.conflicts.len() + self.gone.len() + self.skipped.len()
    }
}

fn list(title: &str, note: &str, items: &[Failure], blocks: &mut Vec<Block>) {
    if items.is_empty() {
        return;
    }
    blocks.push(Block::Heading {
        level: 2,
        text: Inline::plain(title),
    });
    blocks.push(Block::Paragraph(Inline::plain(note)));
    let items = items
        .iter()
        .map(|f| {
            let name = f.path.chars().count();
            let text = format!("{}: {}", f.path, f.reason);
            ListItem::new(vec![Block::Paragraph(Inline::new(
                text,
                vec![Mark::new(MarkKind::Strong, 0, name)],
            ))])
        })
        .collect();
    blocks.push(Block::List {
        ordered: false,
        start: 1,
        items,
    });
}

/// The page listing `notes`.
pub fn problems_document(notes: &Notes) -> Document {
    let mut blocks = vec![Block::Heading {
        level: 1,
        text: Inline::plain("Import problems"),
    }];
    list(
        "Not imported",
        "These files could not be imported and are not in the wiki.",
        &notes.failures,
        &mut blocks,
    );
    if !notes.conflicts.is_empty() {
        blocks.push(Block::Heading {
            level: 2,
            text: Inline::plain("Changed in the file and in the wiki"),
        });
        blocks.push(Block::Paragraph(Inline::plain(
            "The wiki keeps its text. Nothing was overwritten; the version found in the file is shown so that nothing is lost.",
        )));
    }
    for c in &notes.conflicts {
        let place = c.text_path.as_deref().map_or_else(
            || "the text was deleted".to_string(),
            |p| format!("text {p}"),
        );
        blocks.push(Block::Paragraph(Inline::plain(format!(
            "{} ({place}): {}",
            c.file, c.reason
        ))));
        blocks.push(Block::Code {
            lang: "markdown".into(),
            text: c.file_text.clone(),
        });
    }
    list(
        "Files that are gone",
        "These files were imported once. Their texts are kept.",
        &notes.gone,
        &mut blocks,
    );
    list(
        "Not looked at",
        "These entries of the folder were left out.",
        &notes.skipped,
        &mut blocks,
    );
    Document::new("Import problems", blocks)
}
