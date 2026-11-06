//! Real documents survive the Markdown round trip: no word is lost and a second round is stable.
//!
//! Reads the documentation tree read-only; run with `cargo test --test real_docs -- --ignored
//! --nocapture`. Set `FS_DOCS_DIR` to point somewhere else.

use fs_text::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DEFAULT_DIR: &str = "/srv/FreeSynergy/fs-documentation/docs/en";

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
    let dir = std::env::var("FS_DOCS_DIR").unwrap_or_else(|_| DEFAULT_DIR.into());
    let mut files = Vec::new();
    markdown_files(Path::new(&dir), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no Markdown files under {dir}");
    let (mut lossy, mut unstable) = (Vec::new(), Vec::new());
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        let d1 = import_markdown(&src);
        let out1 = export_markdown(&d1, ExportOptions::default());
        let d2 = import_markdown(&out1);
        let out2 = export_markdown(&d2, ExportOptions::default());
        let lost = missing(&src, &out1);
        if !lost.is_empty() {
            lossy.push(format!("{}: {:?}", f.display(), &lost[..lost.len().min(8)]));
        }
        if out1 != out2 || !d1.same_content(&d2) {
            unstable.push(f.display().to_string());
        }
    }
    println!(
        "files: {}, losing words: {}, unstable: {}",
        files.len(),
        lossy.len(),
        unstable.len()
    );
    for l in lossy.iter().take(20) {
        println!("lossy {l}");
    }
    for u in unstable.iter().take(20) {
        println!("unstable {u}");
    }
    assert!(lossy.is_empty() && unstable.is_empty());
}
