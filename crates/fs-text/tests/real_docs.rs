//! Real documents survive the Markdown round trip: no word is lost and a second round is stable.
//!
//! Works on a COPY of the documentation tree only: `cp -r .../fs-documentation/docs/en /tmp/x/`,
//! then `FS_DOCS_DIR=/tmp/x/en cargo test --test real_docs -- --ignored --nocapture`.
//! Without `FS_DOCS_DIR` the test does nothing (never reads the shared tree).

mod common;
use fs_text::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            markdown_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

/// What a reader sees: the rendered page without its tags. (Words of the Markdown source would
/// count link labels, entity names and list numbers, which are syntax and may be written
/// differently.)
fn visible(md: &str) -> String {
    let (mut out, mut in_tag) = (String::new(), false);
    for c in render(md).chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn words(s: &str) -> HashMap<String, i64> {
    let mut m = HashMap::new();
    for w in s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        *m.entry(w.to_string()).or_insert(0) += 1;
    }
    m
}

/// The extensions the importer reads, rendered to HTML: what a reader sees.
fn render(s: &str) -> String {
    use pulldown_cmark::Options as O;
    let o = O::ENABLE_TABLES
        | O::ENABLE_FOOTNOTES
        | O::ENABLE_STRIKETHROUGH
        | O::ENABLE_TASKLISTS
        | O::ENABLE_HEADING_ATTRIBUTES
        | O::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | O::ENABLE_WIKILINKS;
    let mut h = String::new();
    pulldown_cmark::html::push_html(&mut h, pulldown_cmark::Parser::new_ext(s, o));
    h
}

/// Does a table have a row with more cells than its header? GFM drops those cells when it
/// renders; the model keeps them (text a person typed is not dropped), so the pages differ.
fn has_wide_row(src: &str) -> bool {
    let cells = |l: &str| l.replace("\\|", "").matches('|').count();
    let lines: Vec<&str> = src.lines().collect();
    lines.windows(2).enumerate().any(|(i, w)| {
        w[0].trim_start().starts_with('|')
            && w[1].trim_start().starts_with("|-")
            && lines[i + 2..]
                .iter()
                .take_while(|l| l.trim_start().starts_with('|'))
                .any(|l| cells(l) > cells(w[0]))
    })
}

/// Words that occur more often in `a` than in `b`.
fn missing(a: &str, b: &str) -> Vec<String> {
    let (wa, wb) = (words(a), words(b));
    let mut v: Vec<String> = wa
        .iter()
        .filter(|(w, n)| wb.get(*w).copied().unwrap_or(0) < **n)
        .map(|(w, _)| w.clone())
        .collect();
    v.sort();
    v
}

#[test]
#[ignore = "reads the real documentation tree; run with --ignored --nocapture"]
fn real_docs_round_trip_is_faithful_and_stable() {
    let Ok(dir) = std::env::var("FS_DOCS_DIR") else {
        println!("FS_DOCS_DIR not set (use a copy of the docs); skipped");
        return;
    };
    let mut files = Vec::new();
    markdown_files(Path::new(&dir), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no Markdown files under {dir}");
    let (mut lossy, mut unstable, mut changed) = (Vec::new(), Vec::new(), Vec::new());
    let mut rendered = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let d1 = import_markdown(&src);
        if export_markdown(&d1, ExportOptions::default()) != src {
            changed.push(f.display().to_string());
        }
        // the canonical writer alone (no kept source text)
        let out1 = common::canonical(&d1);
        let d2 = import_markdown(&out1);
        let out2 = common::canonical(&d2);
        // wide tables aside (GFM drops cells the model keeps), nothing a reader sees is lost
        let lost = if has_wide_row(&src) {
            Vec::new()
        } else {
            missing(&visible(&src), &visible(&out1))
        };
        if !lost.is_empty() {
            lossy.push(format!("{}: {:?}", f.display(), &lost[..lost.len().min(8)]));
        }
        // the page looks the same (except where GFM drops the cells of a wide table)
        if render(&src) != render(&out1) && !has_wide_row(&src) {
            rendered.push(f.display().to_string());
        }
        if out1 != out2 || !d1.same_content(&d2) {
            unstable.push(f.display().to_string());
        }
    }
    println!(
        "files: {}, not byte-identical untouched: {}, losing words: {}, unstable: {}",
        files.len(),
        changed.len(),
        lossy.len(),
        unstable.len()
    );
    for r in rendered.iter().take(20) {
        println!("renders differently {r}");
    }
    for l in lossy.iter().take(20) {
        println!("lossy {l}");
    }
    for u in unstable.iter().take(20) {
        println!("unstable {u}");
    }
    assert!(changed.is_empty() && lossy.is_empty() && unstable.is_empty() && rendered.is_empty());
}
