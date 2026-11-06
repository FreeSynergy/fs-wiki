//! Example: measure Markdown fidelity of a directory tree (read-only).
//!
//! Usage: `fidelity <dir>...` (use a copy of the docs, never the shared tree). Prints how many
//! files come back byte-identical from an untouched import-export, the changed lines in total,
//! how many files render differently, what one changed word does to the rest of the file and
//! the slowest import-export.
//!
//! Pattern: Facade usage — only the crate root API.
use fs_text::*;
use pulldown_cmark::{html::push_html, Parser};
use std::time::Instant;

fn md_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            md_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

/// The extensions the importer reads.
fn options() -> pulldown_cmark::Options {
    use pulldown_cmark::Options as O;
    O::ENABLE_TABLES
        | O::ENABLE_FOOTNOTES
        | O::ENABLE_STRIKETHROUGH
        | O::ENABLE_TASKLISTS
        | O::ENABLE_HEADING_ATTRIBUTES
        | O::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | O::ENABLE_WIKILINKS
}

fn render(s: &str) -> String {
    let mut h = String::new();
    push_html(&mut h, Parser::new_ext(s, options()));
    h
}

fn changed_lines(a: &str, b: &str) -> usize {
    similar::TextDiff::from_lines(a, b)
        .iter_all_changes()
        .filter(|c| c.tag() != similar::ChangeTag::Equal)
        .count()
}

/// Replace one word of the first plain paragraph; `None` if there is none.
fn edit_one_word(d: &mut Document) -> bool {
    for b in &mut d.blocks {
        if b.attrs.kind != BlockKind::Paragraph || !b.marks.is_empty() {
            continue;
        }
        let t = b.text.to_string();
        let chars: Vec<char> = t.chars().collect();
        let Some(s) = chars.iter().position(|c| c.is_alphabetic()) else {
            continue;
        };
        let e = chars[s..]
            .iter()
            .position(|c| !c.is_alphabetic())
            .map_or(chars.len(), |n| s + n);
        b.splice(s, e, "zzword");
        return true;
    }
    false
}

fn main() {
    let mut files = Vec::new();
    for a in std::env::args().skip(1) {
        md_files(std::path::Path::new(&a), &mut files);
    }
    files.sort();
    let (mut n, mut ident, mut lines, mut render_diff) = (0, 0, 0, 0);
    let (mut edits, mut edit_lines, mut edit_ident_rest) = (0usize, 0usize, 0usize);
    let (mut canon_ident, mut canon_lines, mut canon_render, mut canon_unstable) = (0, 0, 0, 0);
    let (mut slow, mut slow_name) = (0.0f64, String::new());
    for p in &files {
        let Ok(src) = std::fs::read_to_string(p) else {
            continue;
        };
        n += 1;
        let t = Instant::now();
        let d = import_markdown(&src);
        let out = export_markdown(&d, ExportOptions::default());
        let el = t.elapsed().as_secs_f64();
        if el > slow {
            slow = el;
            slow_name = p.display().to_string();
        }
        if out == src {
            ident += 1;
        } else {
            lines += changed_lines(&src, &out);
            if render(&src) != render(&out) {
                render_diff += 1;
            }
        }
        // the canonical writer alone: what a block gets that was changed
        let mut plain = d.clone();
        for b in &mut plain.blocks {
            b.attrs.source = None;
        }
        let canon = export_markdown(&plain, ExportOptions::default());
        if !d.same_content(&import_markdown(&canon)) {
            canon_unstable += 1;
            if std::env::var_os("SHOWR").is_some() {
                println!("  unstable: {}", p.display());
            }
        }
        if canon == src {
            canon_ident += 1;
        } else {
            canon_lines += changed_lines(&src, &canon);
            if render(&src) != render(&canon) {
                canon_render += 1;
                if std::env::var_os("SHOWR").is_some() {
                    println!("  render differs: {}", p.display());
                }
            }
        }
        let mut d2 = d.clone();
        if edit_one_word(&mut d2) {
            edits += 1;
            let out2 = export_markdown(&d2, ExportOptions::default());
            let c = changed_lines(&src, &out2);
            edit_lines += c;
            if c <= 2 {
                edit_ident_rest += 1;
            } else if std::env::var_os("SHOW").is_some() {
                println!("  {c} entries: {}", p.display());
            }
        }
    }
    println!(
        "files {n}, byte-identical {ident} ({:.1}%)",
        100.0 * f64::from(ident) / f64::from(n.max(1))
    );
    println!(
        "changed lines (untouched round trip): {lines}, files rendering differently: {render_diff}"
    );
    println!("canonical writer alone: byte-identical {canon_ident}, changed lines {canon_lines}, files rendering differently {canon_render}, not same_content again {canon_unstable}");
    println!("one-word edits: {edits}, changed lines in total {edit_lines}, edits that change exactly one line (2 diff entries): {edit_ident_rest}");
    println!("slowest: {slow:.3}s {slow_name}");
}
