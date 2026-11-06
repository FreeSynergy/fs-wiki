mod common;
use fs_text::*;
use std::path::PathBuf;

fn corpus() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

fn all_diffs(a: &str, b: &str) -> String {
    let d = similar::TextDiff::from_lines(a, b);
    let mut s = String::new();
    let mut n = 0;
    for c in d.iter_all_changes() {
        match c.tag() {
            similar::ChangeTag::Equal => {}
            t => {
                n += 1;
                if n <= 8 {
                    s.push_str(&format!(
                        "   {} {}\n",
                        if t == similar::ChangeTag::Delete {
                            "-"
                        } else {
                            "+"
                        },
                        c.value().trim_end()
                    ));
                }
            }
        }
    }
    format!("{n} changed lines\n{s}")
}

fn first_diff(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  src: {x}\n  out: {y}", i + 1);
        }
    }
    format!(
        "length differs: {} vs {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

/// md -> model -> md -> model: the second model must equal the first (stable), and
/// the export is reported as identical or normalised.
#[test]
fn corpus_roundtrip() {
    let mut identical = 0;
    let mut normalised = Vec::new();
    for (name, src) in corpus() {
        let d1 = import_markdown(&src);
        assert_eq!(
            export_markdown(&d1, ExportOptions::default()),
            src,
            "{name}: an untouched file must come back byte for byte"
        );
        // the canonical writer alone (no kept source text)
        let out = common::canonical(&d1);
        let d2 = import_markdown(&out);
        assert!(
            d1.same_content(&d2),
            "{name}: model unstable after export; {}",
            first_diff(&src, &out)
        );
        let out2 = common::canonical(&d2);
        assert_eq!(out, out2, "{name}: export not idempotent");
        let mut plain = d1.clone();
        for b in &mut plain.blocks {
            b.attrs.source = None;
        }
        let adm = export_markdown(
            &plain,
            ExportOptions {
                admonitions: true,
                ..Default::default()
            },
        );
        if out == src || adm == src {
            identical += 1;
        } else {
            normalised.push(format!("{name}: {}", all_diffs(&src, &adm)));
        }
    }
    println!("identical: {identical}, normalised: {}", normalised.len());
    for n in &normalised {
        println!("NORMALISED {n}");
    }
}
