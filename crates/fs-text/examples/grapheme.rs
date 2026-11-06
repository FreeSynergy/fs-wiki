//! Example: grapheme-safe marking and editing (ADR 0043 pending improvement).
//!
//! Pattern: Factory (Command) — `Document::edit_op` / `Block::add_mark` snap to clusters.
use fs_text::*;

fn main() {
    let mut doc = Document::new();
    let mut block = Block::paragraph("Hi 👍🏽!");
    // chars: H i ' ' 👍 🏽 ! — offset 4 is inside the emoji; the mark grows to cover it whole
    block.add_mark(Mark::new(MarkKind::Strong, 4, 5, MarkValue::None));
    let id = block.id;
    doc.blocks.push(block);
    let m = &doc.blocks[0].marks.as_slice()[0];
    println!("marked: {:?}", doc.blocks[0].text.slice(m.start..m.end));
    let op = doc.delete_op(id, 4, 5).expect("valid range");
    doc.apply(&op).expect("applies");
    println!("after delete: {:?}", doc.blocks[0].text.to_string());
}
