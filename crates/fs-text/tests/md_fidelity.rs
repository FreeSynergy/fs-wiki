//! Canonical writer fidelity found by an independent review (blockers and should-fix items).
//! Every test here reproduces one reported failure; the canonical writer is used on purpose
//! (no kept source text), so the writer itself is under test.

mod common;
use common::{assert_stable, canon_rt};
use fs_text::*;

fn first(src: &str) -> Block {
    import_markdown(src).blocks.remove(0)
}

// ---- blockers ----

#[test]
fn empty_image_and_empty_link_survive() {
    assert_eq!(canon_rt("a ![](i.png) b\n"), "a ![](i.png) b\n");
    assert_eq!(canon_rt("a [](u) b\n"), "a [](u) b\n");
    assert_eq!(canon_rt("![](i.png)\n"), "![](i.png)\n");
}

#[test]
fn tab_nested_bullets_stay_nested() {
    let d = import_markdown("- a\n\t- b\n\t- c\n- d\n");
    let depth: Vec<usize> = d.blocks.iter().map(|b| b.attrs.containers.len()).collect();
    assert_eq!(depth, [1, 2, 2, 1]);
    let out = common::canonical(&d);
    assert_eq!(out, "- a\n  - b\n  - c\n- d\n");
    assert_eq!(common::canon_rt(&out), out);
}

#[test]
fn link_definitions_and_comment_idiom_are_kept() {
    let src = "para\n\n[unused]: /url \"secret title\"\n\n[//]: # (a private comment)\n\nend\n";
    let out = assert_stable(src);
    assert!(out.contains("[unused]: /url \"secret title\""), "{out:?}");
    assert!(out.contains("[//]: # (a private comment)"), "{out:?}");
    assert_eq!(canon_rt(&out), out);
}

// ---- should fix ----

#[test]
fn escaped_text_stays_text() {
    let b = first("\\[a\\]: b\n");
    assert_eq!(b.text.to_string(), "[a]: b");
    let out = assert_stable("\\[a\\]: b\n");
    assert_eq!(import_markdown(&out).blocks.len(), 1);
    // a literal `!` before a link stays a `!` and a link; all-literal stays all text
    let d = import_markdown(&assert_stable("\\![a](b)\n"));
    assert_eq!(d.blocks[0].marks.iter().next().unwrap().kind, MarkKind::Link);
    let d = import_markdown(&assert_stable("\\!\\[a\\](b)\n"));
    assert!(d.blocks[0].marks.is_empty(), "turned into an image or a link");
    assert_eq!(d.blocks[0].text.to_string(), "![a](b)");
    let d = import_markdown(&assert_stable("\\| a \\|\n\\|---\\|\n"));
    assert_eq!(d.blocks[0].attrs.kind, BlockKind::Paragraph);
    let d = import_markdown(&assert_stable("\\~a\\~\n"));
    assert!(d.blocks[0].marks.is_empty(), "turned into a strikethrough");
    let d = import_markdown(&assert_stable("# T \\{#x\\}\n"));
    assert_eq!(d.blocks[0].attrs.heading_attrs, None);
    assert_eq!(d.blocks[0].text.to_string(), "T {#x}");
}

#[test]
fn thematic_break_at_file_start_is_not_front_matter() {
    let d = import_markdown("***\na: b\n***\n");
    assert_ne!(d.blocks[0].attrs.kind, BlockKind::FrontMatter);
    let out = assert_stable("***\na: b\n***\n");
    assert_ne!(import_markdown(&out).blocks[0].attrs.kind, BlockKind::FrontMatter);
}

#[test]
fn bom_before_front_matter() {
    let d = import_markdown("\u{feff}---\ntitle: x\n---\n\nbody\n");
    assert_eq!(d.blocks[0].attrs.kind, BlockKind::FrontMatter);
}

#[test]
fn html_content_is_not_rewritten() {
    for src in [
        "<!--\n!!! note\n    secret\n-->\n",
        "<pre>\n!!! note\n    secret\n</pre>\n",
    ] {
        let d = import_markdown(src);
        assert_eq!(d.blocks.len(), 1, "{src:?}");
        assert_eq!(d.blocks[0].attrs.kind, BlockKind::Html);
        assert_eq!(d.blocks[0].text.to_string(), src.trim_end());
        assert_eq!(canon_rt(src), src);
    }
}

#[test]
fn plain_lines_are_not_callouts() {
    for src in ["!!! Danger\nnext\n", "??? what\n", "!!! note\nno indent\n"] {
        let d = import_markdown(src);
        assert!(d.blocks.iter().all(|b| b.attrs.containers.is_empty()), "{src:?}");
        assert_stable(src);
    }
}

#[test]
fn escaped_callout_marker_is_text() {
    let d = import_markdown("> \\[!NOTE] x\n");
    assert!(matches!(d.blocks[0].attrs.containers[0], Container::Quote { .. }));
    assert_eq!(d.blocks[0].text.to_string(), "[!NOTE] x");
    assert_stable("> \\[!NOTE] x\n");
    assert!(matches!(
        import_markdown("> [!NOTE] x\n").blocks[0].attrs.containers[0],
        Container::Callout { .. }
    ));
}

#[test]
fn table_row_wider_than_header_keeps_markup() {
    let src = "| a | b |\n|---|---|\n| 1 | 2 | **b** [l](u) |\n";
    let d = import_markdown(src);
    let kinds: Vec<MarkKind> = d.blocks[0].marks.iter().map(|m| m.kind).collect();
    assert!(kinds.contains(&MarkKind::Strong) && kinds.contains(&MarkKind::Link), "{kinds:?}");
    let out = assert_stable(src);
    assert!(out.contains("**b**") && out.contains("[l](u)"), "{out:?}");
}

#[test]
fn link_targets_do_not_change() {
    for src in [
        "[a](x&amp;copy;y)\n",
        "[a](b\\*c)\n",
        "[a](b\\\\*c)\n",
        "[a](<b c>)\n",
        "[a](b \"t\\\\\")\n",
    ] {
        let d1 = import_markdown(src);
        let out = common::canonical(&d1);
        let d2 = import_markdown(&out);
        let path = |d: &Document| d.blocks[0].marks.iter().next().and_then(|m| m.link().cloned());
        assert_eq!(path(&d1), path(&d2), "{src:?} -> {out:?}");
    }
    assert_eq!(
        import_markdown("[a](x&amp;copy;y)\n").blocks[0].marks.iter().next().unwrap().link().unwrap().path,
        "x&copy;y"
    );
}

#[test]
fn code_fence_with_backtick_in_info() {
    let src = "~~~ a`b\ncode\n~~~\n";
    let d = import_markdown(src);
    assert_eq!(d.blocks[0].attrs.kind, BlockKind::Code { info: "a`b".into() });
    assert_stable(src);
}

#[test]
fn empty_footnote_definition_is_kept() {
    let out = assert_stable("text[^1]\n\n[^1]:\n");
    assert!(out.contains("[^1]:"), "{out:?}");
}

#[test]
fn tabs_and_separators_in_text() {
    assert_eq!(canon_rt("a\tb\n"), "a\tb\n");
    let d = import_markdown("| a |\n|---|\n| x&#9;y |\n| p&#10;q |\n");
    assert_eq!(d.blocks[0].text.to_string(), "a\nx y\np q");
    assert_stable("| a |\n|---|\n| x&#9;y |\n");
    let d = import_markdown("a\u{2028}b\n");
    assert!(!d.blocks[0].text.to_string().contains('\u{2028}'));
    assert!(!canonical_has_break(&d));
}

fn canonical_has_break(d: &Document) -> bool {
    common::canonical(d).contains("\\\n")
}

#[test]
fn fallback_escapes_everything() {
    // whatever the text is, the canonical form reads back as the same text
    for t in ["| a |\n|---|", "~a~", "![a](b)", "[a]: b", "# T {#x}", "<b>x</b>", "1. x", "- y"] {
        let mut d = Document::new();
        d.blocks.push(Block::paragraph(t));
        let out = common::canonical(&d);
        let d2 = import_markdown(&out);
        assert!(d.same_content(&d2), "{t:?} -> {out:?}");
    }
}

#[test]
fn many_marks_are_fast() {
    let src = "*a* ".repeat(25_000);
    let t = std::time::Instant::now();
    let out = canon_rt(&src);
    let secs = t.elapsed().as_secs_f64();
    assert!(out.contains("*a*"));
    assert!(secs < 20.0, "100 KB of marks took {secs} s");
}
