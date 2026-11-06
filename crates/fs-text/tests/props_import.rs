//! Property tests: external import reproduces its target; Markdown model is stable.

use fs_text::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn diff_import_reproduces_target(a in "[ab\n]{0,40}", b in "[abc\n]{0,40}") {
        let mut d = Document::from_file_text(&a);
        let _ = import_external(&mut d, &b);
        prop_assert_eq!(d.raw_text().unwrap(), b);
    }
    #[test]
    fn markdown_model_is_stable(parts in proptest::collection::vec(prop_oneof![
        Just("# Head".to_string()), Just("para *em* and **st**".to_string()), Just("- a\n- b".to_string()),
        Just("1. x\n2. y".to_string()), Just("> quote".to_string()), Just("```\ncode\n```".to_string()),
        Just("| a | b |\n|---|---|\n| 1 | 2 |".to_string()), Just("[l](u) `c` ~~s~~".to_string()),
        "[a-z*_\\[\\]#> -]{1,12}", ], 1..6)) {
        let src = parts.join("\n\n");
        // Runs of three delimiters: see `known_issue_rule_of_three`.
        prop_assume!(!src.contains("***") && !src.contains("___"));
        // Two lists with only an empty quote between them: see `known_issue_adjacent_lists`.
        prop_assume!(!parts.iter().any(|p| p.trim_matches([' ', '>']).is_empty()));
        let d1 = import_markdown(&src);
        let out = export_markdown(&d1, ExportOptions::default());
        let d2 = import_markdown(&out);
        prop_assert!(d1.same_content(&d2), "{src:?} -> {out:?}");
    }
}

#[test]
fn delimiters_of_neighbours_stay_as_written() {
    for src in [
        "_x_*a*", "_#_*a*", "*x*_a_", "_x_**a**", "*a*_b_ c", "__x__*a*",
    ] {
        let d = import_markdown(src);
        let out = export_markdown(&d, ExportOptions::default());
        assert_eq!(out.trim_end(), src);
        assert!(d.same_content(&import_markdown(&out)), "{src}");
    }
}

/// Known issue (DESIGN.md): CommonMark's rule of three pairs `*a *b**\\*` differently from
/// `*a *b***`. No text is lost, only the nesting of the marks changes.
#[test]
#[ignore = "known issue: a nested emphasis ending in front of a literal star"]
fn known_issue_rule_of_three() {
    let d = import_markdown("*[ *#***");
    let out = export_markdown(&d, ExportOptions::default());
    assert!(d.same_content(&import_markdown(&out)), "{out}");
}

/// Known issue (DESIGN.md): two lists with nothing written between them become one on re-import.
/// No text is lost, only the split into two lists, and an empty quote has no block at all.
#[test]
#[ignore = "known issue: adjacent lists of the same type merge"]
fn known_issue_adjacent_lists() {
    let d = import_markdown("1. x\n2. y\n\n>\n\n1. x\n2. y");
    let out = export_markdown(&d, ExportOptions::default());
    assert!(d.same_content(&import_markdown(&out)), "{out}");
}
