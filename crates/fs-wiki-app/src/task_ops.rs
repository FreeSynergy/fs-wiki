//! Ticking a checkbox as an ordinary change of the text model.
//!
//! Pattern: Command — the number of a box (document order, the same numbering as `fs_components_widgets::count_tasks`) becomes
//! the operations that set it: `SetAttrs` on the block of a list item (the task state is an attribute of the item, ADR 0084),
//! an `Edit` of the one character between the brackets for a table cell (a box in a cell is our extension, GFM has task items
//! in lists only; the cell stays the text `[ ]` / `[x]`). Every other byte stays: a block that is not changed keeps its
//! source text, the changed block is written by the writer.

use std::collections::HashSet;

use fs_text::{BlockKind, Container, Document, Op};

/// Why a box could not be ticked.
#[derive(Debug, PartialEq, Eq)]
pub struct NoSuchBox(pub usize);

/// The task state of a cell text: `Some(done)` for `[ ]`, `[x]`, `[X]` alone or followed by a space.
fn cell_task(cell: &str) -> Option<bool> {
    let done = match cell.get(..3)? {
        "[ ]" => false,
        "[x]" | "[X]" => true,
        _ => return None,
    };
    cell[3..]
        .is_empty()
        .then_some(done)
        .or_else(|| cell[3..].starts_with(' ').then_some(done))
}

/// The operations that set box number `index` to `done`; none when it already has that state.
///
/// # Errors
/// [`NoSuchBox`] when the document has fewer boxes.
pub fn tick_ops(doc: &Document, index: usize, done: bool) -> Result<Vec<Op>, NoSuchBox> {
    let mut n = 0;
    let mut items: HashSet<u32> = HashSet::new();
    for block in &doc.blocks {
        if let BlockKind::Table { .. } = &block.attrs.kind {
            let text = block.text.to_string();
            let mut at = 0;
            for line in text.split('\n') {
                for cell in line.split('\t') {
                    if let Some(now) = cell_task(cell) {
                        if n == index {
                            return Ok(if now == done {
                                Vec::new()
                            } else {
                                vec![Op::Edit {
                                    block: block.id,
                                    start: at + 1,
                                    end: at + 2,
                                    insert: if done { "x" } else { " " }.to_owned(),
                                    restore: None,
                                }]
                            });
                        }
                        n += 1;
                    }
                    at += cell.chars().count() + 1;
                }
            }
        } else {
            let own = block
                .attrs
                .containers
                .iter()
                .rposition(|c| matches!(c, Container::Item { .. }));
            let Some(i) = own else { continue };
            let Container::Item {
                inst,
                task: Some(now),
                ..
            } = &block.attrs.containers[i]
            else {
                continue;
            };
            if !items.insert(*inst) {
                continue; // a later block of the same item
            }
            if n == index {
                if *now == done {
                    return Ok(Vec::new());
                }
                let mut attrs = block.attrs.clone();
                if let Container::Item { task, .. } = &mut attrs.containers[i] {
                    *task = Some(done);
                }
                // The block is written again by the writer; its old source text would still say the old state.
                attrs.source = None;
                return Ok(vec![Op::SetAttrs {
                    block: block.id,
                    attrs,
                }]);
            }
            n += 1;
        }
    }
    Err(NoSuchBox(index))
}
