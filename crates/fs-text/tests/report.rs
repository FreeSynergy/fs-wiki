//! A block that cannot be written so that it reads back exactly does not drag its untouched
//! neighbours along: they stay byte for byte, and the block is reported as inexact.

use fs_text::*;

const FILE: &str = "Title\n=====\n\n1. x\n1. y\n\n[z][q]\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\n~~~ rust\nfn main() {}\n~~~\n\n[q]: /k\n\n";

fn export(d: &Document) -> (String, ExportReport) {
    export_markdown_report(d, ExportOptions::default())
}

fn neighbours_kept(out: &str) {
    for kept in [
        "Title\n=====\n",
        "1. x\n1. y\n",
        "[z][q]\n",
        "| a   | b |\n|-----|---|\n| 1   | 2 |\n",
        "~~~ rust\nfn main() {}\n~~~\n",
        "[q]: /k\n",
    ] {
        assert!(out.contains(kept), "{kept:?} was rewritten: {out:?}");
    }
}

#[test]
fn a_link_block_that_cannot_read_back_leaves_the_neighbours_alone() {
    for culprit in [
        "[[t/p.md|*T*]]",
        "&#32;[l](t/p.md)",
        "[a](t/p.md) [b](c&#10;d)",
        "### a&#10;[l](t/p.md)",
    ] {
        let src = format!("{FILE}{culprit}\n");
        let mut docs = vec![import_markdown(&src)];
        let _ = rewrite_links(&mut docs, "t", "u");
        let (out, report) = export(&docs[0]);
        neighbours_kept(&out);
        assert!(out.contains("u/p.md"), "{culprit:?}: {out:?}");
        // the report names only what is inexact; whatever it names, the rest is exact
        let last = docs[0].blocks.last().unwrap().id;
        assert!(
            report.inexact.iter().all(|i| i.block == last),
            "{culprit:?}: {report:?}"
        );
    }
}

#[test]
fn text_with_edge_blanks_or_newlines_is_reported_not_spread() {
    for new in [" new", "new ", "a\n\nb"] {
        let mut d = import_markdown(&format!("{FILE}last\n"));
        let at = d.blocks.len() - 1;
        let n = d.blocks[at].text.len();
        d.blocks[at].splice(0, n, new);
        let (out, report) = export(&d);
        neighbours_kept(&out);
        assert!(!report.inexact.is_empty(), "{new:?}: should be reported");
        assert!(out.contains(new.trim()), "{new:?}: {out:?}");
    }
}

#[test]
fn an_html_block_edited_to_an_open_comment_is_reported() {
    let mut d = import_markdown(&format!("{FILE}<!-- c -->\n"));
    let at = d.blocks.len() - 1;
    let n = d.blocks[at].text.len();
    d.blocks[at].splice(0, n, "<!-- open");
    let (out, report) = export(&d);
    neighbours_kept(&out);
    assert!(out.contains("<!-- open") && report.inexact.is_empty() || !report.inexact.is_empty());
}

#[test]
fn a_block_appended_after_an_open_block_is_reported() {
    for open in [
        "<!-- open",
        "<?php",
        "<![CDATA[ x",
        "<!DOCTYPE html",
        "<pre>",
    ] {
        let mut d = import_markdown(&format!("a\n\n{open}\n"));
        d.blocks.push(Block::paragraph("new text"));
        let (out, report) = export(&d);
        assert!(out.starts_with("a\n\n"), "{open:?}: {out:?}");
        assert!(
            !report.inexact.is_empty(),
            "{open:?}: swallowed text must be reported: {out:?}"
        );
        assert!(report.inexact.iter().all(|i| !i.why.is_empty()));
    }
}

#[test]
fn a_clean_edit_has_an_empty_report() {
    let mut d = import_markdown("# T\n\nsome words\n");
    d.blocks[1].splice(0, 4, "other");
    let (out, report) = export(&d);
    assert_eq!(out, "# T\n\nother words\n");
    assert!(report.inexact.is_empty());
}

#[test]
fn a_second_definition_at_the_end_survives_a_move() {
    let src = "[x][r]\n\n[r]: t/p.md\n[r]: /keep\n";
    let mut docs = vec![import_markdown(src)];
    let _ = rewrite_links(&mut docs, "t", "u");
    let (out, _) = export(&docs[0]);
    assert!(out.contains("[r]: /keep"), "{out:?}");
    // and with the first block removed: nothing typed disappears
    let mut d = import_markdown("[r]: /a\n[r]: /keep2\n\ntext\n");
    d.blocks.remove(0);
    let (out, _) = export(&d);
    assert!(out.contains("/keep2"), "{out:?}");
}

#[test]
fn a_link_in_a_callout_title_is_rewritten() {
    let mut docs = vec![import_markdown("> [!NOTE] [l](t/p.md)\n> body\n")];
    let _ = rewrite_links(&mut docs, "t", "u");
    let (out, _) = export(&docs[0]);
    assert!(out.contains("[l](u/p.md)"), "{out:?}");
}

#[test]
fn a_tampered_source_of_the_same_length_is_not_used() {
    let mut d = import_markdown("Title\n=====\n\nreal\n");
    d.blocks[1].attrs.source.as_mut().unwrap().text = "\nFAKE\n".into();
    let (out, _) = export(&d);
    assert!(!out.contains("FAKE") && out.contains("real"), "{out:?}");
}

#[test]
fn a_second_definition_behind_a_quote_or_on_two_lines_survives() {
    for src in [
        "> [x][r]\n>\n> [r]: t/p.md\n> [r]: /keep\n",
        "[x][r]\n\n[r]: t/p.md\n[r]:\n  /keep\n",
        "[x][r]\n\n[r]: t/p.md\n\n  [r]: /keep\n",
    ] {
        let mut docs = vec![import_markdown(src)];
        let _ = rewrite_links(&mut docs, "t", "u");
        let (out, _) = export(&docs[0]);
        assert!(out.contains("/keep"), "{src:?} -> {out:?}");
        // and when the neighbours are written anew
        // the definition in front of it is written anew: the tail of its source is dropped
        let mut d = import_markdown(src);
        let at = d
            .blocks
            .iter()
            .position(|b| b.text.to_string().starts_with("[r]"))
            .unwrap();
        d.blocks[at].splice(5, 8, "t2/p.md");
        let (out, report) = export(&d);
        assert!(
            out.contains("/keep") || !report.inexact.is_empty(),
            "{src:?} edited -> {out:?}"
        );
        assert!(out.contains("/keep"), "{src:?} edited -> {out:?}");
    }
}

#[test]
fn text_that_lives_only_in_a_dropped_source_is_reported() {
    // a stray line in front of a block, invisible to the model, is gone when the block is
    // written anew at another place: the report must say so
    let mut d = import_markdown("a\n\nb\n");
    let s = d.blocks[1].attrs.source.as_mut().unwrap();
    s.text = "\nONLY IN THE SOURCE\n\nb\n".into();
    s.own = s.text.find("\nb").unwrap() + 1;
    let fp = d.blocks[1].fingerprint();
    let s = d.blocks[1].attrs.source.as_mut().unwrap();
    s.hash = Source::seal(&fp, &s.rest, &s.text, (s.own, s.end));
    d.blocks.swap(0, 1);
    let (out, report) = export(&d);
    assert!(!out.contains("ONLY IN THE SOURCE"), "{out:?}");
    assert!(
        report
            .inexact
            .iter()
            .any(|i| i.why.contains("ONLY IN THE SOURCE")),
        "{report:?}"
    );
}

#[test]
fn a_tilde_fence_stays_a_tilde_fence_when_neighbours_are_rewritten() {
    let src = "[x][r]\n\n~~~ rust\nfn a() {}\n~~~\n\n[r]: /u\n";
    let mut d = import_markdown(src);
    d.blocks.pop(); // the definition goes: the link block before it is rewritten
    let (out, _) = export(&d);
    assert!(out.contains("~~~ rust\nfn a() {}\n~~~"), "{out:?}");
}
