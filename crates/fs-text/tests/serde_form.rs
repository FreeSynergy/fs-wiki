//! The JSON form of a document (what the wiki stores): every mark kind, points and ranges, block
//! attributes and containers survive a round trip, also through `serde_json::Value`.

use fs_text::*;

fn marked() -> Document {
    let mut d = import_markdown(
        "---\ntitle: T\n---\n\n# Head\n\nsome **bold** [[a/b|x]] and `code` text[^1]\n\n> quote with ![img](p.png)\n\n- a\n- b\n\n| h1 | h2 |\n|----|----|\n| c1 | c2 |\n\n[^1]: note\n",
    );
    let b = &mut d.blocks[2];
    b.add_mark(Mark::new(
        MarkKind::Passage,
        0,
        4,
        MarkValue::Name("warning".into()),
    ));
    b.add_mark(Mark::new(
        MarkKind::Explanation,
        5,
        9,
        MarkValue::Explanation {
            replacement: "my own words".into(),
        },
    ));
    b.add_mark(Mark::point(
        MarkKind::Variable,
        3,
        MarkValue::Variable {
            name: "who".into(),
            ty: Some("contact".into()),
        },
    ));
    d
}

#[test]
fn every_kind_of_mark_survives_json() {
    let d = marked();
    let kinds: std::collections::HashSet<_> = d
        .blocks
        .iter()
        .flat_map(|b| b.marks.iter().map(|m| m.kind))
        .collect();
    for k in [
        MarkKind::Strong,
        MarkKind::Code,
        MarkKind::Link,
        MarkKind::Image,
        MarkKind::FootnoteRef,
        MarkKind::Passage,
        MarkKind::Explanation,
        MarkKind::Variable,
    ] {
        assert!(kinds.contains(&k), "the fixture lacks {k:?}");
    }

    let text = serde_json::to_string(&d).unwrap();
    let back: Document = serde_json::from_str(&text).unwrap();
    assert_eq!(back, d, "text form");

    let value = serde_json::to_value(&d).unwrap();
    let back: Document = serde_json::from_value(value).unwrap();
    assert_eq!(back, d, "value form");
    for (a, b) in d.blocks.iter().zip(&back.blocks) {
        assert_eq!(a.marks.as_slice(), b.marks.as_slice());
        assert_eq!(a.attrs, b.attrs);
        assert_eq!(a.text.to_string(), b.text.to_string());
    }
}

#[test]
fn a_restored_document_still_edits_and_exports_the_same() {
    let d = marked();
    let mut back: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(
        export_markdown(&back, ExportOptions::default()),
        export_markdown(&d, ExportOptions::default())
    );
    // The restored marks still follow edits: delete the text of the passage, the mark goes.
    let id = back.blocks[2].id;
    let op = Op::Edit {
        block: id,
        start: 0,
        end: 4,
        insert: String::new(),
        restore: None,
    };
    back.apply(&op).unwrap();
    assert!(back.blocks[2]
        .marks
        .iter()
        .all(|m| m.kind != MarkKind::Passage));
}
