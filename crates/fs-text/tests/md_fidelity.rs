//! Canonical writer fidelity found by an independent review (blockers and should-fix items).
//! Every test here reproduces one reported failure; the canonical writer is used on purpose
//! (no kept source text), so the writer itself is under test.

mod common;
use common::{assert_stable, canon_rt};
use fs_text::*;

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
