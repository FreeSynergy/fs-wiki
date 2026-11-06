//! The pages that did not import, as a page of their own.
//!
//! Pattern: Adapter — the list of failures becomes a document in the neutral model, so the
//! workbench shows it like any page and nothing is hidden.

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

/// The page listing `failures`.
pub fn problems_document(failures: &[Failure]) -> Document {
    let mut blocks = vec![
        Block::Heading {
            level: 1,
            text: Inline::plain("Import problems"),
        },
        Block::Paragraph(Inline::plain(format!(
            "{} file(s) could not be imported and are not in the wiki.",
            failures.len()
        ))),
    ];
    let items = failures
        .iter()
        .map(|f| {
            let name = f.path.chars().count();
            let text = format!("{}: {}", f.path, f.reason);
            ListItem {
                task: None,
                blocks: vec![Block::Paragraph(Inline::new(
                    text,
                    vec![Mark::new(MarkKind::Strong, 0, name)],
                ))],
            }
        })
        .collect();
    blocks.push(Block::List {
        ordered: false,
        start: 1,
        items,
    });
    Document {
        title: "Import problems".into(),
        blocks,
    }
}
