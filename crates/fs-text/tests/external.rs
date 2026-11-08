//! External edits become ops that keep layers.

use fs_text::*;
mod common;

#[allow(clippy::many_single_char_names)] // short fixture names (a, b, d, s)
#[test]
fn external_edit_moves_marks() {
    let mut d = Document::from_file_text("fn main() {\n    let name = 1;\n}\n");
    let id = d.blocks[0].id;
    // layer marks: passage over `name`, variable point after `=`
    let s = 20;
    assert_eq!(d.blocks[0].text.slice(s..s + 4), "name");
    d.blocks[0].marks.add(Mark::new(
        MarkKind::Passage,
        s,
        s + 4,
        MarkValue::Name("n".into()),
    ));
    d.blocks[0].marks.add(Mark::point(
        MarkKind::Variable,
        s + 7,
        MarkValue::Variable {
            name: "v".into(),
            ty: None,
        },
    ));
    let new = "// header\nfn main() {\n    let name = 1;\n    let other = 2;\n}\n";
    let (ops, inv) = import_external(&mut d, new);
    assert_ne!(ops, [] as [fs_text::Op; 0]);
    assert_eq!(d.raw_text().unwrap(), new);
    let b = &d.blocks[0];
    let p = b
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Passage)
        .unwrap();
    assert_eq!(b.text.slice(p.start..p.end), "name");
    let v = b
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Variable)
        .unwrap();
    assert_eq!(b.text.slice(v.start - 2..v.start), "= ");
    // external import is undoable like any edit
    for op in &inv {
        d.apply(op).unwrap();
    }
    assert_eq!(d.raw_text().unwrap(), "fn main() {\n    let name = 1;\n}\n");
    let p = d.blocks[0]
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Passage)
        .unwrap();
    assert_eq!(d.blocks[0].text.slice(p.start..p.end), "name");
    let _ = id;
}

#[test]
fn external_edit_deleting_text_removes_its_marks() {
    let mut d = Document::from_file_text("keep this and drop that end");
    d.blocks[0].marks.add(Mark::new(
        MarkKind::Passage,
        14,
        24,
        MarkValue::Name("x".into()),
    ));
    let (_, inv) = import_external(&mut d, "keep this end");
    assert!(d.blocks[0].marks.is_empty());
    for op in &inv {
        d.apply(op).unwrap();
    }
    assert_eq!(d.blocks[0].marks.len(), 1);
}

#[test]
fn external_markdown_import_keeps_layers() {
    let mut d = import_markdown("# Title\n\nFirst paragraph with text.\n\nSecond paragraph.\n");
    // layer on the first paragraph
    let b1 = d.blocks[1].id;
    d.blocks[1].marks.add(Mark::new(
        MarkKind::Passage,
        6,
        15,
        MarkValue::Name("para".into()),
    ));
    let ids: Vec<_> = d.blocks.iter().map(|b| b.id).collect();
    let new =
        "# Title\n\nFirst paragraph with **more** text.\n\nInserted block.\n\nSecond paragraph.\n";
    let _ = import_external(&mut d, new);
    assert_eq!(export_markdown(&d, ExportOptions::default()), new);
    assert_eq!(d.blocks[1].id, b1, "block identity kept");
    assert_eq!(d.blocks[0].id, ids[0]);
    assert_eq!(d.blocks[3].id, ids[2]);
    let b = &d.blocks[1];
    let p = b
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Passage)
        .unwrap();
    assert_eq!(b.text.slice(p.start..p.end), "paragraph");
    assert!(b.marks.iter().any(|m| m.kind == MarkKind::Strong));
}
