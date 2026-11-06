//! Example: time import and export of marks-heavy documents.
//!
//! Usage: `speed [kilobytes]`. Prints seconds for the import, the canonical export (what a
//! changed block costs) and the export of an untouched document (kept source texts).
//!
//! Pattern: Facade usage — only the crate root API.
use fs_text::*;
use std::time::Instant;

fn run(name: &str, src: &str) {
    let t = Instant::now();
    let d = import_markdown(src);
    let import = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let same = export_markdown(&d, ExportOptions::default()) == src;
    let untouched = t.elapsed().as_secs_f64();
    let mut plain = d.clone();
    for b in &mut plain.blocks {
        b.attrs.source = None;
    }
    let t = Instant::now();
    let _ = export_markdown(&plain, ExportOptions::default());
    let canonical = t.elapsed().as_secs_f64();
    println!(
        "{name:28} {:>8} bytes  import {import:7.3}s  canonical export {canonical:7.3}s  untouched export {untouched:7.3}s  identical {same}",
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
