use fs_text::*;
use proptest::prelude::*;

fn block_with(text: &str, marks: Vec<Mark>) -> (Document, uuid::Uuid) {
    let mut b = Block::paragraph(text);
    for m in marks {
        b.marks.add(m);
    }
    let id = b.id;
    let mut d = Document::new();
    d.blocks.push(b);
    (d, id)
}

fn marks_of(d: &Document) -> Vec<Mark> {
    d.blocks[0].marks.as_slice().to_vec()
}

#[test]
fn expand_rules() {
    // strong expands after, not before; link expands neither way
    let (mut d, id) = block_with("ab cd", vec![Mark::new(MarkKind::Strong, 3, 5, MarkValue::None), Mark::new(MarkKind::Link, 0, 2, MarkValue::Link(LinkValue::new("x", LinkStyle::Inline)))]);
    d.apply(&Op::insert_text(id, 5, "!")).unwrap(); // end of strong: joins
    d.apply(&Op::insert_text(id, 3, "_")).unwrap(); // start of strong: does not join
    d.apply(&Op::insert_text(id, 2, "?")).unwrap(); // end of link: does not join
    let b = &d.blocks[0];
    let strong = b.marks.iter().find(|m| m.kind == MarkKind::Strong).unwrap();
    let link = b.marks.iter().find(|m| m.kind == MarkKind::Link).unwrap();
    assert_eq!(b.text.slice(strong.start..strong.end), "cd!");
    assert_eq!(b.text.slice(link.start..link.end), "ab");
}

#[test]
fn point_marks_and_deletion() {
    let (mut d, id) = block_with("hello world", vec![Mark::point(MarkKind::Variable, 5, MarkValue::Variable { name: "x".into(), ty: None }), Mark::new(MarkKind::Passage, 6, 11, MarkValue::Name("w".into()))]);
    // deleting only part of "world" keeps the passage
    d.apply(&Op::delete_text(id, 6, 8)).unwrap();
    assert_eq!(marks_of(&d).len(), 2);
    // deleting the rest removes it with the text
    let inv = d.apply(&Op::delete_text(id, 5, 9)).unwrap();
    assert!(d.blocks[0].marks.iter().all(|m| m.kind != MarkKind::Passage));
    // the point at 5 stays (boundary), whole-text deletion is undoable
    d.apply(&inv).unwrap();
    assert_eq!(d.blocks[0].text.to_string(), "hello rld");
    assert_eq!(marks_of(&d).len(), 2);
    // a point strictly inside a deleted range dies
    let (mut d, id) = block_with("abcdef", vec![Mark::point(MarkKind::Variable, 3, MarkValue::None)]);
    d.apply(&Op::delete_text(id, 1, 5)).unwrap();
    assert!(marks_of(&d).is_empty());
}

#[test]
fn merge_and_unmark() {
    let mut s = MarkSet::new();
    s.add(Mark::new(MarkKind::Strong, 0, 3, MarkValue::None));
    s.add(Mark::new(MarkKind::Strong, 2, 6, MarkValue::None));
    assert_eq!(s.len(), 1);
    s.add(Mark::new(MarkKind::Link, 0, 3, MarkValue::Link(LinkValue::new("a", LinkStyle::Inline))));
    s.add(Mark::new(MarkKind::Link, 1, 4, MarkValue::Link(LinkValue::new("a", LinkStyle::Inline))));
    assert_eq!(s.len(), 3, "links never merge");
    s.unmark(MarkKind::Strong, 2, 4);
    let strongs: Vec<_> = s.iter().filter(|m| m.kind == MarkKind::Strong).map(|m| (m.start, m.end)).collect();
    assert_eq!(strongs, vec![(0, 2), (4, 6)]);
}

#[test]
fn registry_is_complete() {
    for s in REGISTRY {
        assert_eq!(s.kind.spec().name, s.name);
    }
    assert_eq!(MarkKind::Variable.spec().point, true);
}

#[test]
fn anchors() {
    let a = Anchor::new(5, Bias::Left);
    assert_eq!(a.after_insert(5, 2).offset, 5);
    assert_eq!(Anchor::new(5, Bias::Right).after_insert(5, 2).offset, 7);
    assert_eq!(a.after_delete(3, 8).offset, 3);
    assert_eq!(Anchor::new(9, Bias::Left).after_delete(3, 8).offset, 4);
}

#[test]
fn serde_roundtrip() {
    let d = import_markdown("# T\n\nsome **bold** [[a/b|x]] text\n\n- a\n- b\n");
    let j = serde_json::to_string(&d).unwrap();
    let d2: Document = serde_json::from_str(&j).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn history_undo_redo_and_retention() {
    let (mut d, id) = block_with("hello world", vec![Mark::new(MarkKind::Passage, 6, 11, MarkValue::Name("w".into()))]);
    let orig = d.clone();
    let mut h = History::new(Retention { max_age: None, max_steps: Some(3) });
    h.apply(&mut d, vec![Op::delete_text(id, 5, 11)], "cut", 100).unwrap();
    assert_eq!(d.blocks[0].text.to_string(), "hello");
    assert!(h.undo(&mut d).unwrap());
    assert_eq!(d, orig, "undo restores text and the dead mark");
    assert!(h.redo(&mut d).unwrap());
    assert_eq!(d.blocks[0].text.to_string(), "hello");
    assert!(h.undo(&mut d).unwrap());
    // retention by steps: 5 steps, only 3 undoable, pending ops kept for sync
    for i in 0..5 {
        h.apply(&mut d, vec![Op::insert_text(id, 0, &i.to_string())], "type", 200 + i).unwrap();
    }
    assert_eq!(h.len(), 3);
    assert_eq!(h.pending_sync().len(), 2, "compacted steps stay until synced");
    let current = d.clone();
    while h.undo(&mut d).unwrap() {}
    assert_ne!(d, current);
    while h.redo(&mut d).unwrap() {}
    assert_eq!(d, current);
    h.mark_synced();
    assert!(h.pending_sync().is_empty());
    // retention by age
    let mut h = History::new(Retention { max_age: Some(60), max_steps: None });
    h.apply(&mut d, vec![Op::insert_text(id, 0, "a")], "old", 0).unwrap();
    h.apply(&mut d, vec![Op::insert_text(id, 0, "b")], "new", 1000).unwrap();
    assert_eq!(h.len(), 1);
    let j = serde_json::to_string(&h).unwrap();
    let h2: History = serde_json::from_str(&j).unwrap();
    assert_eq!(h, h2);
}

#[test]
fn failed_step_rolls_back() {
    let (mut d, id) = block_with("abc", vec![]);
    let before = d.clone();
    let mut h = History::new(Retention::default());
    let r = h.apply(&mut d, vec![Op::insert_text(id, 0, "x"), Op::delete_text(id, 0, 99)], "bad", 0);
    assert!(r.is_err());
    assert_eq!(d, before);
}

#[test]
fn external_edit_moves_marks() {
    let mut d = Document::from_file_text("fn main() {\n    let name = 1;\n}\n");
    let id = d.blocks[0].id;
    // layer marks: passage over `name`, variable point after `=`
    let s = 20;
    assert_eq!(d.blocks[0].text.slice(s..s + 4), "name");
    d.blocks[0].marks.add(Mark::new(MarkKind::Passage, s, s + 4, MarkValue::Name("n".into())));
    d.blocks[0].marks.add(Mark::point(MarkKind::Variable, s + 7, MarkValue::Variable { name: "v".into(), ty: None }));
    let new = "// header\nfn main() {\n    let name = 1;\n    let other = 2;\n}\n";
    let (ops, inv) = import_external(&mut d, new);
    assert!(!ops.is_empty());
    assert_eq!(d.raw_text().unwrap(), new);
    let b = &d.blocks[0];
    let p = b.marks.iter().find(|m| m.kind == MarkKind::Passage).unwrap();
    assert_eq!(b.text.slice(p.start..p.end), "name");
    let v = b.marks.iter().find(|m| m.kind == MarkKind::Variable).unwrap();
    assert_eq!(b.text.slice(v.start - 2..v.start), "= ");
    // external import is undoable like any edit
    for op in &inv {
        d.apply(op).unwrap();
    }
    assert_eq!(d.raw_text().unwrap(), "fn main() {\n    let name = 1;\n}\n");
    let p = d.blocks[0].marks.iter().find(|m| m.kind == MarkKind::Passage).unwrap();
    assert_eq!(d.blocks[0].text.slice(p.start..p.end), "name");
    let _ = id;
}

#[test]
fn external_edit_deleting_text_removes_its_marks() {
    let mut d = Document::from_file_text("keep this and drop that end");
    d.blocks[0].marks.add(Mark::new(MarkKind::Passage, 14, 24, MarkValue::Name("x".into())));
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
    d.blocks[1].marks.add(Mark::new(MarkKind::Passage, 6, 15, MarkValue::Name("para".into())));
    let ids: Vec<_> = d.blocks.iter().map(|b| b.id).collect();
    let new = "# Title\n\nFirst paragraph with **more** text.\n\nInserted block.\n\nSecond paragraph.\n";
    import_external(&mut d, new);
    assert_eq!(export_markdown(&d, ExportOptions::default()), new);
    assert_eq!(d.blocks[1].id, b1, "block identity kept");
    assert_eq!(d.blocks[0].id, ids[0]);
    assert_eq!(d.blocks[3].id, ids[2]);
    let b = &d.blocks[1];
    let p = b.marks.iter().find(|m| m.kind == MarkKind::Passage).unwrap();
    assert_eq!(b.text.slice(p.start..p.end), "paragraph");
    assert!(b.marks.iter().any(|m| m.kind == MarkKind::Strong));
}

#[test]
fn link_table_and_rewrite() {
    let mut a = import_markdown("See [[wiki/old-page]] and [[wiki/old-page#Goals]] and [x](wiki/old-page/sub) and [ext](https://e.org/wiki/old-page).\n\nShown [[wiki/old-page|label]].\n");
    let rows = extract_links(&a);
    assert_eq!(rows.len(), 5);
    assert!(rows[0].internal && !rows[3].internal);
    assert_eq!(rows[0].title, "wiki/old-page");
    let mut docs = vec![a.clone()];
    let changes = rewrite_links(&mut docs, "wiki/old-page", "archive/new");
    assert_eq!(changes.len(), 1);
    let out = export_markdown(&docs[0], ExportOptions::default());
    assert_eq!(out, "See [[archive/new]] and [[archive/new#Goals]] and [x](archive/new/sub) and [ext](https://e.org/wiki/old-page).\n\nShown [[archive/new|label]].\n");
    // undo
    for op in &changes[0].inverse {
        docs[0].apply(op).unwrap();
    }
    assert!(docs[0].same_content(&a));
    a.blocks.clear();
    assert_eq!(moved_path_check(), Some("b/c#h".to_string()));
}
fn moved_path_check() -> Option<String> {
    fs_text::links::moved_path("a/c#h", "a", "b")
}

#[test]
fn templates() {
    let now = DateTime::from_unix(1_793_000_000); // 2026-10-26
    assert_eq!(now.format("%Y-%m-%d %A").unwrap(), "2026-10-26 Monday");
    assert_eq!(DateTime::from_unix(0).format("%Y-%m-%d %a %j").unwrap(), "1970-01-01 Thu 001");
    let v = Values::new(now).set("title", "Plan");
    assert_eq!(instantiate_str("# {{title}} ({{date:%d.%m.%Y}}) \\{{keep}}", &v).unwrap(), "# Plan (26.10.2026) {{keep}}");
    assert_eq!(instantiate_str("{{missing}}", &v), Err(TemplateError::Missing("missing".into())));
    assert_eq!(fs_text::template::variables("{{b}} {{a}} {{date:%Y}} {{a}}"), vec!["a", "b"]);

    // document template: bold placeholder keeps its bold, variable point gets filled
    let mut t = import_markdown("Dear **{{name}}**, welcome.\n");
    t.blocks[0].marks.add(Mark::point(MarkKind::Variable, 0, MarkValue::Variable { name: "greeting".into(), ty: None }));
    let v = Values::new(now).set("name", "Ada Lovelace").set("greeting", "Hi! ");
    let d = instantiate(&t, &v).unwrap();
    assert_eq!(d.blocks[0].text.to_string(), "Hi! Dear Ada Lovelace, welcome.");
    let strong = d.blocks[0].marks.iter().find(|m| m.kind == MarkKind::Strong).unwrap();
    assert_eq!(d.blocks[0].text.slice(strong.start..strong.end), "Ada Lovelace");
    let var = d.blocks[0].marks.iter().find(|m| m.kind == MarkKind::Variable).unwrap();
    assert_eq!(d.blocks[0].text.slice(var.start..var.end), "Hi! ");
    assert_ne!(d.id, t.id);
}

#[test]
fn markdown_semantics() {
    let d = import_markdown("> [!note]+ Hello *there*\n> Body\n\n!!! warning \"Careful\"\n    Mind the gap.\n\nText[^a] and ![[img.png]] ^blk\n\n[^a]: Note.\n");
    use Container::*;
    assert!(matches!(&d.blocks[0].attrs.containers[0], Callout { ty, title, fold: Some('+'), .. } if ty == "note" && title == "Hello there"));
    assert_eq!(d.blocks[0].text.to_string(), "Body");
    assert!(matches!(&d.blocks[1].attrs.containers[0], Callout { ty, title, .. } if ty == "warning" && title == "Careful"));
    let p = &d.blocks[2];
    assert_eq!(p.attrs.block_ref.as_deref(), Some("blk"));
    assert!(p.marks.iter().any(|m| m.kind == MarkKind::FootnoteRef && m.is_point()));
    assert!(p.marks.iter().any(|m| m.kind == MarkKind::Image && m.link().unwrap().style == LinkStyle::Embed));
    assert!(matches!(&d.blocks[3].attrs.containers[0], Footnote { label, .. } if label == "a"));
}

#[test]
fn layers_are_not_exported_by_default() {
    let mut d = import_markdown("Hello world\n");
    d.blocks[0].marks.add(Mark::new(MarkKind::Passage, 0, 5, MarkValue::Name("greet".into())));
    d.blocks[0].marks.add(Mark::point(MarkKind::Variable, 5, MarkValue::Variable { name: "who".into(), ty: None }));
    assert_eq!(export_markdown(&d, ExportOptions::default()), "Hello world\n");
    assert_eq!(export_markdown(&d, ExportOptions { layers: true }), "[Hello]{#greet}{{who}} world\n");
}

#[test]
fn escaping_roundtrips() {
    for src in ["a \\* b \\_c\\_ \\`x\\` \\[y\\](z)", "1\\. not a list", "\\# not a heading", "\\> not a quote", "\\- not a bullet", "snake_case_name and 2 * 3", "\\<div> &amp; \\&amp;", "tilde \\~\\~x\\~\\~"] {
        let d = import_markdown(src);
        let out = export_markdown(&d, ExportOptions::default());
        let d2 = import_markdown(&out);
        assert!(d.same_content(&d2), "{src:?} -> {out:?}");
    }
}

// ---------- property tests ----------

#[derive(Clone, Debug)]
enum E {
    Ins(usize, String),
    Del(usize, usize),
}

fn edit_strategy() -> impl Strategy<Value = Vec<E>> {
    proptest::collection::vec(prop_oneof![(0usize..40, "[a-zé ]{0,5}").prop_map(|(a, s)| E::Ins(a, s)), (0usize..40, 0usize..8).prop_map(|(a, n)| E::Del(a, a + n))], 0..25)
}

fn mark_strategy() -> impl Strategy<Value = Vec<(usize, usize, u8)>> {
    proptest::collection::vec((0usize..30, 0usize..10, 0u8..4), 0..6)
}

fn build(text: &str, ms: &[(usize, usize, u8)]) -> (Document, uuid::Uuid) {
    let len = text.chars().count();
    let marks = ms
        .iter()
        .map(|(s, n, k)| {
            let s = (*s).min(len);
            let e = (s + n).min(len);
            match k {
                0 => Mark::new(MarkKind::Strong, s, e, MarkValue::None),
                1 => Mark::new(MarkKind::Passage, s, e, MarkValue::Name("p".into())),
                2 => Mark::point(MarkKind::Variable, s, MarkValue::None),
                _ => Mark::new(MarkKind::Link, s, e, MarkValue::Link(LinkValue::new("x", LinkStyle::Inline))),
            }
        })
        .collect();
    block_with(text, marks)
}

fn apply_e(d: &mut Document, id: uuid::Uuid, e: &E) -> Op {
    let len = d.blocks[0].text.len();
    let op = match e {
        E::Ins(a, s) => Op::insert_text(id, (*a).min(len), s),
        E::Del(a, b) => Op::delete_text(id, (*a).min(len), (*b).min(len)),
    };
    d.apply(&op).unwrap()
}

proptest! {
    #[test]
    fn marks_stay_valid(text in "[a-zé ]{0,30}", ms in mark_strategy(), es in edit_strategy()) {
        let (mut d, id) = build(&text, &ms);
        for e in &es {
            apply_e(&mut d, id, e);
            let len = d.blocks[0].text.len();
            for m in d.blocks[0].marks.iter() {
                prop_assert!(m.start <= m.end && m.end <= len, "{m:?} len {len}");
                if m.kind == MarkKind::Variable { prop_assert!(m.is_point()); }
            }
        }
    }

    #[test]
    fn undo_restores_everything(text in "[a-zé ]{0,30}", ms in mark_strategy(), es in edit_strategy()) {
        let (mut d, id) = build(&text, &ms);
        let orig = d.clone();
        let mut inverses = Vec::new();
        for e in &es { inverses.push(apply_e(&mut d, id, e)); }
        for inv in inverses.iter().rev() { d.apply(inv).unwrap(); }
        prop_assert_eq!(d, orig);
    }

    #[test]
    fn edits_outside_a_mark_do_not_change_its_text(text in "[a-z ]{10,30}", s in 2usize..8, n in 1usize..6, ins in "[a-z]{0,4}", at in 0usize..40, left in any::<bool>()) {
        // expand = None mark (passage): text typed before/after/outside never joins it
        let len = text.chars().count();
        let (s, e) = (s.min(len), (s + n).min(len));
        let (mut d, id) = block_with(&text, vec![Mark::new(MarkKind::Passage, s, e, MarkValue::Name("p".into()))]);
        let before = d.blocks[0].text.slice(s..e);
        let at = if left { at.min(s) } else { at.clamp(e, len.max(e)) };
        d.apply(&Op::insert_text(id, at.min(len), &ins)).unwrap();
        let m = d.blocks[0].marks.iter().next().unwrap();
        prop_assert_eq!(d.blocks[0].text.slice(m.start..m.end), before);
    }

    #[test]
    fn diff_import_reproduces_target(a in "[ab\n]{0,40}", b in "[abc\n]{0,40}") {
        let mut d = Document::from_file_text(&a);
        import_external(&mut d, &b);
        prop_assert_eq!(d.raw_text().unwrap(), b);
    }

    #[test]
    fn markdown_model_is_stable(parts in proptest::collection::vec(prop_oneof![
        Just("# Head".to_string()), Just("para *em* and **st**".to_string()), Just("- a\n- b".to_string()),
        Just("1. x\n2. y".to_string()), Just("> quote".to_string()), Just("```\ncode\n```".to_string()),
        Just("| a | b |\n|---|---|\n| 1 | 2 |".to_string()), Just("[l](u) `c` ~~s~~".to_string()),
        "[a-z*_\\[\\]#> -]{1,12}", ], 1..6)) {
        let src = parts.join("\n\n");
        let d1 = import_markdown(&src);
        let out = export_markdown(&d1, ExportOptions::default());
        let d2 = import_markdown(&out);
        prop_assert!(d1.same_content(&d2), "{src:?} -> {out:?}");
    }
}
