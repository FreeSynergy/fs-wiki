//! Example: time import and export of marks-heavy documents.
//!
//! Usage: `speed [kilobytes]`. Prints seconds for the import, the canonical export (what a
//! changed block costs) and the export of an untouched document (kept source texts).
//!
//! Pattern: Facade usage — only the crate root API.
use fs_text::{export_markdown, import_markdown, ExportOptions};
use std::time::Instant;

fn run(name: &str, src: &str) {
    let t = Instant::now();
    let d = import_markdown(src);
    let import = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let same = export_markdown(&d, ExportOptions::default()) == src;
    let untouched = t.elapsed().as_secs_f64();
    // one word changed in the first block: the rest is written from kept sources, then proven
    let mut edited = d.clone();
    if let Some(b) = edited.blocks.first_mut() {
        let n = b.text.len();
        b.splice(0, n.min(1), "X");
    }
    let t = Instant::now();
    let _ = export_markdown(&edited, ExportOptions::default());
    let one_word = t.elapsed().as_secs_f64();
    let mut plain = d.clone();
    for b in &mut plain.blocks {
        b.attrs.source = None;
    }
    let t = Instant::now();
    let _ = export_markdown(&plain, ExportOptions::default());
    let canonical = t.elapsed().as_secs_f64();
    println!(
        "{name:28} {:>8} bytes  import {import:7.3}s  canonical export {canonical:7.3}s  untouched export {untouched:7.3}s  one-word edit export {one_word:7.3}s  identical {same}",
        src.len()
    );
}

fn main() {
    let kb: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(100);
    let n = kb * 1024 / 4;
    run("`*a* ` repeated", &"*a* ".repeat(n));
    run(
        "`# - ` then `*a* ` repeated",
        &format!("# - {}", "*a* ".repeat(n)),
    );
    run("`[l](u) ` repeated", &"[l](u) ".repeat(n * 4 / 7));
    run(
        "paragraphs with marks",
        &"some *em* and **st** text\n\n".repeat(n / 6),
    );
    let row = "| *a* | **b** | [c](d) |\n";
    run(
        "table",
        &format!("| h1 | h2 | h3 |\n|---|---|---|\n{}", row.repeat(n / 7)),
    );
}
