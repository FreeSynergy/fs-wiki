//! Shared helpers for the integration tests.
#![allow(dead_code)]

use fs_text::{Block, Document, Mark};

/// A one-paragraph document with the given marks.
pub fn block_with(text: &str, marks: Vec<Mark>) -> (Document, uuid::Uuid) {
    let mut b = Block::paragraph(text);
    for m in marks {
        b.marks.add(m);
    }
    let id = b.id;
    let mut d = Document::new();
    d.blocks.push(b);
    (d, id)
}

/// Marks of the first block.
pub fn marks_of(d: &Document) -> Vec<Mark> {
    d.blocks[0].marks.as_slice().to_vec()
}
