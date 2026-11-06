//! Round-trip fixes: runs of delimiters, adjacent lists, empty quotes, wide tables, plain text.

mod common;
use fs_text::*;

/// The canonical writer (no kept source text): these tests are about the writer itself.
fn export(d: &Document) -> String {
    common::canonical(d)
}

fn stable(src: &str) -> String {
    let d = import_markdown(src);
    let out = export(&d);
    assert!(common::reads_back_as(&d, &out), "{src:?} -> {out:?}");
    out
}

#[test]
fn runs_of_three_delimiters_read_back() {
    for src in [
        "*[ *#***",
        "a#___[_]_",
        "**a*]*",
        "*a*__a__",
        "[[a]]_#_",
        "***a***",
        "*a **b***",
        "*>_]_*_a_",
        "*_a_*_a_",
    ] {
        stable(src);
    }
}

#[test]
fn plain_text_stays_clean() {
    for src in [
        "2 * 3 = 6",
        "snake_case and a_b and a_b_c",
        "a * b * c",
        "1 _ 2",
        "x - y + z",
        "wait... what? (yes)",
    ] {
        let out = export(&import_markdown(src));
        assert_eq!(out.trim_end(), src);
    }
}

#[test]
fn an_empty_quote_is_a_block() {
    let d = import_markdown(">");
    assert_eq!(d.blocks.len(), 1);
    assert!(matches!(
        d.blocks[0].attrs.containers[..],
        [Container::Quote { .. }]
    ));
    assert_eq!(export(&d), ">\n");
    assert_eq!(
        export_markdown(&d, ExportOptions::default()),
        ">",
        "unchanged: as read"
    );
    let nested = import_markdown("> >");
    assert_eq!(nested.blocks.len(), 1);
    assert_eq!(nested.blocks[0].attrs.containers.len(), 2);
    stable("> >");
    stable("> a\n>\n> >");
}

#[test]
fn adjacent_lists_stay_two_lists() {
    let src = "1. x\n2. y\n\n>\n\n1. x\n2. y";
    stable(src);
    // two lists with a link definition between them: the definition is a block (kept)
    let d = import_markdown("- a\n\n[x]: /u\n\n- b");
    assert_eq!(d.blocks.len(), 3);
    assert_ne!(
        d.blocks[0].attrs.containers[0].inst(),
        d.blocks[2].attrs.containers[0].inst()
    );
    assert_eq!(common::canonical(&d), "- a\n\n[x]: /u\n\n- b\n");
    // two lists with nothing between them: the separator comment is not a block
    let d = import_markdown("- a\n\n<!-- -->\n\n- b");
    assert_ne!(
        d.blocks[0].attrs.containers[0].inst(),
        d.blocks[1].attrs.containers[0].inst()
    );
    let out = common::canonical(&d);
    assert_eq!(out, "- a\n\n<!-- -->\n\n- b\n");
    let back = import_markdown(&out);
    assert_eq!(back.blocks.len(), 2, "the separator is not a block");
    assert!(d.same_content(&back));
    // a different marker needs no separator
    assert_eq!(stable("- a\n\n* b").trim_end(), "- a\n\n* b");
}

#[test]
fn comments_that_are_not_separators_stay() {
    for src in [
        "<!-- -->",
        "a\n\n<!-- -->\n\nb",
        "- a\n\n<!-- note -->\n\n- b",
        "- a\n\n<!-- -->\n\nb",
    ] {
        let d = import_markdown(src);
        assert!(
            d.blocks.iter().any(|b| b.attrs.kind == BlockKind::Html),
            "{src}"
        );
        stable(src);
    }
}

#[test]
fn lists_without_paragraphs_keep_their_blank_lines() {
    assert_eq!(stable("*\n\n* #").trim_end(), "*\n\n* #");
    stable("- a\n- b\n\n  * c");
}

#[test]
fn a_rule_inside_a_dash_item_is_still_a_rule() {
    assert_eq!(stable("- ***").trim_end(), "- ***");
    stable("- - ***");
    stable(">- ***");
}

#[test]
fn table_cells_beyond_the_header_are_kept() {
    let src = "| a | b |\n|---|---|\n| 1 | 2 | 3 |\n| x |\n";
    let d = import_markdown(src);
    assert_eq!(d.blocks[0].text.to_string(), "a\tb\t\n1\t2\t3\nx\t\t");
    let out = stable(src);
    assert!(out.contains("| 1 | 2 | 3 |"), "{out}");
    assert!(import_markdown(&out).same_content(&d));
}
