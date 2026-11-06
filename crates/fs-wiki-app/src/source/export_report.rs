//! What an export could not say exactly, in words for the person.
//!
//! Pattern: Adapter — `ExportReport` of fs-text names blocks by id; the person is told where
//! (the number of the block and its first words) and why, and how many untouched blocks had to
//! be written anew.

use fs_text::{Document, ExportReport};

/// How much of a block's text names it.
const NAME: usize = 30;

/// "This file could not be written exactly: ..." if any block is inexact.
pub(super) fn inexact_message(doc: &Document, report: &ExportReport) -> Option<String> {
    if report.inexact.is_empty() {
        return None;
    }
    let items: Vec<String> = report
        .inexact
        .iter()
        .map(|i| {
            let place = doc.blocks.iter().position(|b| b.id == i.block).map(|n| {
                let words: String = doc.blocks[n].text.to_string().chars().take(NAME).collect();
                format!("block {} (\"{}\")", n + 1, words.replace('\n', " "))
            });
            format!("{}: {}", place.unwrap_or_else(|| "a block".into()), i.why)
        })
        .collect();
    Some(format!(
        "This file could not be written exactly: {}.{}",
        items.join("; "),
        rewritten_note(report).unwrap_or_default()
    ))
}

/// " N untouched block(s) had to be written anew." if there were any.
pub(super) fn rewritten_note(report: &ExportReport) -> Option<String> {
    let n = report.rewritten.len();
    (n > 0).then(|| format!(" {n} untouched block(s) had to be written anew."))
}
