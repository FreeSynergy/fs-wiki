//! Markdown semantics and escaping.

use fs_text::*;
mod common;

#[test]
fn markdown_semantics() {
    let d = import_markdown("> [!note]+ Hello *there*\n> Body\n\n!!! warning \"Careful\"\n    Mind the gap.\n\nText[^a] and ![[img.png]] ^blk\n\n[^a]: Note.\n");
    use Container::*;
    assert!(
        matches!(&d.blocks[0].attrs.containers[0], Callout { ty, title, fold: Some('+'), .. } if ty == "note" && title == "Hello *there*")
    );
    assert_eq!(d.blocks[0].text.to_string(), "Body");
    assert!(
        matches!(&d.blocks[1].attrs.containers[0], Callout { ty, title, .. } if ty == "warning" && title == "Careful")
    );
    let p = &d.blocks[2];
    assert_eq!(p.attrs.block_ref.as_deref(), Some("blk"));
    assert!(p
        .marks
        .iter()
        .any(|m| m.kind == MarkKind::FootnoteRef && m.is_point()));
    assert!(p
        .marks
        .iter()
        .any(|m| m.kind == MarkKind::Image && m.link().unwrap().style == LinkStyle::Embed));
    assert!(matches!(&d.blocks[3].attrs.containers[0], Footnote { label, .. } if label == "a"));
}

#[test]
fn layers_are_not_exported_by_default() {
    let mut d = import_markdown("Hello world\n");
    d.blocks[0].marks.add(Mark::new(
        MarkKind::Passage,
        0,
        5,
        MarkValue::Name("greet".into()),
    ));
    d.blocks[0].marks.add(Mark::point(
        MarkKind::Variable,
        5,
        MarkValue::Variable {
            name: "who".into(),
            ty: None,
        },
    ));
    assert_eq!(
        export_markdown(&d, ExportOptions::default()),
        "Hello world\n"
    );
    assert_eq!(
        export_markdown(
            &d,
            ExportOptions {
                layers: true,
                ..Default::default()
            }
        ),
        "[Hello]{#greet}{{who}} world\n"
    );
}

#[test]
fn escaping_roundtrips() {
    for src in [
        "a \\* b \\_c\\_ \\`x\\` \\[y\\](z)",
        "1\\. not a list",
        "\\# not a heading",
        "\\> not a quote",
        "\\- not a bullet",
        "snake_case_name and 2 * 3",
        "\\<div> &amp; \\&amp;",
        "tilde \\~\\~x\\~\\~",
    ] {
        let d = import_markdown(src);
        let out = export_markdown(&d, ExportOptions::default());
        let d2 = import_markdown(&out);
        assert!(d.same_content(&d2), "{src:?} -> {out:?}");
    }
}
