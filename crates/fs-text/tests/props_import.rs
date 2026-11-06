//! Property tests: external import reproduces its target; Markdown model is stable.

mod common;
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
        let d1 = import_markdown(&src);
        // the canonical writer alone: the kept source would only write the file back
        let out = common::canonical(&d1);
        prop_assert!(common::reads_back_as(&d1, &out), "{src:?} -> {out:?}");
        // untouched, the file comes back as it was
        prop_assert_eq!(export_markdown(&d1, ExportOptions::default()), src);
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
