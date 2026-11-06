//! Byte-level round trips: an untouched import-export gives the file back byte for byte, a
//! changed block is written anew and nothing else changes, and a stale source is never written.

mod common;
use fs_text::*;

fn export(d: &Document) -> String {
    export_markdown(d, ExportOptions::default())
}

/// Hand-written file with the styles that used to be normalised.
const SAMPLE: &str = "Title\n=====\n\nSub\n---\n\nSome *text*  \nwith a break and\\\nanother.\n\n1. one\n1. two\n1. three\n\n| a   | b |\n|-----|:-:|\n| 1   | 2 |\n\n!!! note \"Careful\"\n    Mind the gap.\n\n* star\n* bullets\n\n[ref]: /url \"title\"\n\n    indented code\n\n<!-- comment -->\n\nlast line without newline";

fn edit_first_word(d: &mut Document, kind: BlockKind) {
    let b = d.blocks.iter_mut().find(|b| b.attrs.kind == kind).unwrap();
    let t = b.text.to_string();
    let end = t.find(' ').map_or(t.len(), |i| t[..i].chars().count());
    b.splice(0, end, "changed");
}

#[test]
fn untouched_files_come_back_byte_for_byte() {
    let samples = [
        SAMPLE,
        "",
        "\n\n",
        "   \n",
        "\u{feff}---\ntitle: x\n---\n\nbody\n",
        "# a\r\n\r\ntext  \r\nmore\r\n\r\n- x\r\n- y\r\n",
        "- a\n\t- b\n\t- c\n- d\n",
        "- > a\n- b\n",
        "> [!note] T\n> body\n\n??? tip\n    hidden\n",
        "text\n\n\n\n\nmuch space\n\n\n",
        "```rs\nfn main() {}\n```\nglued\n",
        "a\tb\n",
        "[x]: /u\n[y]: /v\n\ntext\n",
        "- a\n\n<!-- -->\n\n- b\n",
        "[^1]: note\n\ntext[^1]\n",
        "text\r",
        "a\u{2028}b\n",
        "***\na: b\n***\n",
    ];
    for s in samples {
        let d = import_markdown(s);
        assert_eq!(export(&d), s, "untouched: {s:?}");
    }
}

#[test]
fn every_corpus_file_is_byte_identical() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let s = std::fs::read_to_string(&p).unwrap();
        assert_eq!(export(&import_markdown(&s)), s, "{}", p.display());
    }
}

#[test]
fn one_changed_word_changes_one_line() {
    let mut d = import_markdown(SAMPLE);
    edit_first_word(&mut d, BlockKind::Paragraph);
    let out = export(&d);
    let changed: Vec<_> = SAMPLE
        .lines()
        .zip(out.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(SAMPLE.lines().count(), out.lines().count());
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].1.starts_with("changed"), "{changed:?}");
    // Setext, `1. 1.`, padding, `!!!`, indented code and the missing final newline survive
    assert!(
        out.contains("Title\n=====") && out.contains("1. three") && out.contains("| a   | b |")
    );
    assert!(out.contains("!!! note \"Careful\"\n    Mind the gap.") && out.ends_with("newline"));
}

#[test]
fn crlf_file_stays_crlf_when_a_block_changes() {
    let src = "# Head\r\n\r\nfirst paragraph\r\n\r\n- a\r\n- b\r\n";
    let mut d = import_markdown(src);
    edit_first_word(&mut d, BlockKind::Paragraph);
    let out = export(&d);
    assert_eq!(out, "# Head\r\n\r\nchanged paragraph\r\n\r\n- a\r\n- b\r\n");
    d.blocks[2].insert(1, "X");
    assert!(
        export(&d).replace("\r\n", "").find('\n').is_none(),
        "mixed line endings"
    );
}

#[test]
fn the_kept_source_survives_json_and_is_not_content() {
    let d = import_markdown(SAMPLE);
    let json = serde_json::to_string(&d).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(export(&back), SAMPLE);
    let value: serde_json::Value = serde_json::to_value(&d).unwrap();
    let back: Document = serde_json::from_value(value).unwrap();
    assert_eq!(export(&back), SAMPLE);
    let mut plain = d.clone();
    for b in &mut plain.blocks {
        b.attrs.source = None;
    }
    assert!(d.same_content(&plain) && plain.same_content(&d));
    assert_ne!(common::canonical(&d), SAMPLE);
}

/// Edit, export and check: the edit is in the output, the output reads back as the edited
/// document, and the blocks the edit did not touch are still written as they were read.
fn check_edit(src: &str, edit: impl Fn(&mut Document), expect: &str) {
    let mut d = import_markdown(src);
    edit(&mut d);
    let out = export(&d);
    assert_eq!(out, expect, "edit of {src:?}");
    let back = import_markdown(&out);
    assert!(d.same_content(&back), "{src:?} -> {out:?}");
}

#[test]
fn a_stale_source_is_never_written() {
    let src = "Title\n=====\n\nsome *marked* text here\n\n- a\n- b\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\nlast\n";
    // text edit
    check_edit(src, |d| d.blocks[1].insert(0, "X"), "Title\n=====\n\nXsome *marked* text here\n\n- a\n- b\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\nlast\n");
    // mark removed
    check_edit(src, |d| {
        let id = d.blocks[1].marks.iter().next().unwrap().id;
        d.blocks[1].marks.remove(id);
    }, "Title\n=====\n\nsome marked text here\n\n- a\n- b\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\nlast\n");
    // mark added
    check_edit(src, |d| {
        d.blocks[1].add_mark(Mark::new(MarkKind::Strong, 0, 4, MarkValue::None));
    }, "Title\n=====\n\n**some** *marked* text here\n\n- a\n- b\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\nlast\n");
    // list item added
    check_edit(src, |d| {
        let mut b = d.blocks[2].clone();
        b.id = fs_text::Block::paragraph("").id;
        b.attrs.source = None;
        b.attrs.extra.insert("glue".into(), "1".into());
        if let Container::Item { inst, .. } = &mut b.attrs.containers[0] {
            *inst = 99;
        }
        b.splice(0, 1, "new");
        d.blocks.insert(3, b);
    }, "Title\n=====\n\nsome *marked* text here\n\n- a\n- new\n- b\n\n| a   | b |\n|-----|---|\n| 1   | 2 |\n\nlast\n");
    // table cell
    check_edit(src, |d| {
        let t = d.blocks[4].text.to_string();
        let at = t.chars().count() - 1;
        d.blocks[4].splice(at, at + 1, "9");
    }, "Title\n=====\n\nsome *marked* text here\n\n- a\n- b\n\n| a | b |\n|-----|---|\n| 1 | 9 |\n\nlast\n");
}
