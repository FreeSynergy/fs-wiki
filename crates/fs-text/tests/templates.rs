//! Templates and date formatting.

use fs_text::*;
mod common;

#[test]
fn templates() {
    let now = DateTime::from_unix(1_793_000_000); // 2026-10-26
    assert_eq!(now.format("%Y-%m-%d %A").unwrap(), "2026-10-26 Monday");
    assert_eq!(
        DateTime::from_unix(0).format("%Y-%m-%d %a %j").unwrap(),
        "1970-01-01 Thu 001"
    );
    let v = Values::new(now).set("title", "Plan");
    assert_eq!(
        instantiate_str("# {{title}} ({{date:%d.%m.%Y}}) \\{{keep}}", &v).unwrap(),
        "# Plan (26.10.2026) {{keep}}"
    );
    assert_eq!(
        instantiate_str("{{missing}}", &v),
        Err(TemplateError::Missing("missing".into()))
    );
    assert_eq!(
        fs_text::template::variables("{{b}} {{a}} {{date:%Y}} {{a}}"),
        vec!["a", "b"]
    );

    // document template: bold placeholder keeps its bold, variable point gets filled
    let mut t = import_markdown("Dear **{{name}}**, welcome.\n");
    t.blocks[0].marks.add(Mark::point(
        MarkKind::Variable,
        0,
        MarkValue::Variable {
            name: "greeting".into(),
            ty: None,
        },
    ));
    let v = Values::new(now)
        .set("name", "Ada Lovelace")
        .set("greeting", "Hi! ");
    let d = instantiate(&t, &v).unwrap();
    assert_eq!(
        d.blocks[0].text.to_string(),
        "Hi! Dear Ada Lovelace, welcome."
    );
    let strong = d.blocks[0]
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Strong)
        .unwrap();
    assert_eq!(
        d.blocks[0].text.slice(strong.start..strong.end),
        "Ada Lovelace"
    );
    let var = d.blocks[0]
        .marks
        .iter()
        .find(|m| m.kind == MarkKind::Variable)
        .unwrap();
    assert_eq!(d.blocks[0].text.slice(var.start..var.end), "Hi! ");
    assert_ne!(d.id, t.id);
}
