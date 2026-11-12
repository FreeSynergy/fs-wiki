//! Example: write the Markdown file given as argument canonically (no kept source text) after importing it.
//!
//! Pattern: Facade usage — only the crate root API. For looking at what the canonical writer changes.
use fs_text::{export_markdown, import_markdown, ExportOptions};
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let s = std::fs::read_to_string(p).unwrap();
    let mut d = import_markdown(&s);
    for b in &mut d.blocks {
        b.attrs.source = None;
    }
    print!("{}", export_markdown(&d, ExportOptions::default()));
}
