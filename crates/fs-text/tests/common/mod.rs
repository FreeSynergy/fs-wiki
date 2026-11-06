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

/// Export without any kept source text: the canonical writer alone.
pub fn canonical(d: &fs_text::Document) -> String {
    let mut d = d.clone();
    for b in &mut d.blocks {
        b.attrs.source = None;
    }
    fs_text::export_markdown(&d, fs_text::ExportOptions::default())
}

/// Import, then write canonically.
pub fn canon_rt(src: &str) -> String {
    canonical(&fs_text::import_markdown(src))
}

/// Canonical round trip must read back to the same content.
pub fn assert_stable(src: &str) -> String {
    let d1 = fs_text::import_markdown(src);
    let out = canonical(&d1);
    let d2 = fs_text::import_markdown(&out);
    assert!(
        d1.same_content(&d2),
        "{src:?} -> {out:?}\nfirst:  {:?}\nsecond: {:?}",
        d1.blocks
            .iter()
            .map(|b| (&b.attrs.kind, b.text.to_string(), b.marks.len()))
            .collect::<Vec<_>>(),
        d2.blocks
            .iter()
            .map(|b| (&b.attrs.kind, b.text.to_string(), b.marks.len()))
            .collect::<Vec<_>>()
    );
    out
}
