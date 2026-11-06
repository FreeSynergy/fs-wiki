//! More review findings: duplicate definitions, empty images in links, block references, callout
//! case and the layout hints of the canonical writer.

mod common;
use fs_text::*;

fn export(d: &Document) -> String {
    export_markdown(d, ExportOptions::default())
}

#[test]
fn a_nbsp_line_is_not_blank() {
    let mut d = import_markdown("a\n\n\u{a0}\n\nb\n");
    d.blocks[2].splice(0, 1, "B");
    assert!(export(&d).contains('\u{a0}'));
}

#[test]
fn a_duplicate_definition_survives_a_move() {
    let src = "[r]: /a\n\n[r]: /keepme\n\n[l](t/p.md)\n";
    let mut docs = vec![import_markdown(src)];
    let _ = rewrite_links(&mut docs, "t", "u");
    let out = export(&docs[0]);
    assert!(
        out.contains("[r]: /keepme") && out.contains("u/p.md"),
        "{out:?}"
    );
}

#[test]
fn a_link_around_an_empty_image_stays_around_it() {
    let src = "[![](i.png)](t/p.md)\n";
    assert_eq!(common::canon_rt(src), src);
    let mut docs = vec![import_markdown(src)];
    let _ = rewrite_links(&mut docs, "t", "uu");
    assert_eq!(export(&docs[0]), "[![](i.png)](uu/p.md)\n");
}

#[test]
fn a_block_reference_does_not_swallow_text() {
    for src in ["Relax to `^1`\n", "E = mc ^2\n"] {
        let d = import_markdown(src);
        assert_eq!(d.blocks[0].attrs.block_ref, None, "{src:?}");
        assert!(d.blocks[0].text.to_string().contains('^'));
    }
    assert_eq!(
        import_markdown("text ^blk-1\n").blocks[0]
            .attrs
            .block_ref
            .as_deref(),
        Some("blk-1")
    );
}

#[test]
fn a_callout_keeps_the_case_of_its_type() {
    let d = import_markdown("> [!NOTE] T\n> body\n");
    assert_eq!(common::canonical(&d), "> [!NOTE] T\n> body\n");
}

#[test]
fn style_hints_are_kept_by_the_canonical_writer() {
    // rule style
    assert_eq!(common::canon_rt("a\n\n***\n\nb\n"), "a\n\n***\n\nb\n");
    // glue: a tight list stays tight, a fence stays glued
    assert_eq!(common::canon_rt("- a\n- b\n"), "- a\n- b\n");
    assert_eq!(
        common::canon_rt("```\nx\n```\ntext\n"),
        "```\nx\n```\ntext\n"
    );
    // admonition dialect
    assert_eq!(
        common::canon_rt("!!! note \"T\"\n    body\n"),
        "!!! note \"T\"\n    body\n"
    );
}
