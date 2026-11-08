//! A kept source text is never written when the file would not read back as the model: the
//! output proves itself and falls back to the canonical writer (changed blocks and their
//! neighbours first, then the whole document).

mod common;
use common::reads_back_as;
use fs_text::*;

fn export(d: &Document) -> String {
    export_markdown(d, ExportOptions::default())
}

fn item(inst: u32) -> Container {
    Container::Item {
        inst,
        list: inst + 1,
        number: None,
        marker: '-',
        tight: true,
        task: None,
    }
}

/// Edit, export; the file must read back as the edited model.
fn edited(src: &str, edit: impl Fn(&mut Document)) -> String {
    let mut d = import_markdown(src);
    edit(&mut d);
    let out = export(&d);
    assert!(reads_back_as(&d, &out), "{src:?} -> {out:?}");
    out
}

#[test]
fn a_removed_definition_does_not_leave_a_literal_reference() {
    edited("[x][r]\n\nmid\n\n[r]: /u\n", |d| {
        d.blocks.pop();
    });
}

#[test]
fn a_changed_definition_is_seen_by_the_link_before_it() {
    edited("[x][r]\n\n[r]: /u\n", |d| d.blocks[1].splice(5, 7, "/v"));
}

#[test]
fn a_block_that_loses_its_container_does_not_drag_the_next_one_along() {
    edited("- x\n\n  y\n", |d| {
        d.blocks[0].attrs.containers.clear();
        // a single item cannot be written loose
        if let Container::Item { tight, .. } = &mut d.blocks[1].attrs.containers[0] {
            *tight = true;
        }
    });
}

#[test]
fn a_block_that_becomes_a_list_item_does_not_swallow_the_code_after_it() {
    edited("x\n\n    code\n", |d| {
        d.blocks[0].attrs.containers.push(item(50));
    });
}

#[test]
fn a_heading_that_becomes_a_paragraph_does_not_merge_with_the_text_below() {
    edited("# h\ntext\n", |d| {
        d.blocks[0].attrs.kind = BlockKind::Paragraph;
    });
}

#[test]
fn html_that_changes_does_not_swallow_the_text_below() {
    edited("<!-- c -->\ntext\n", |d| {
        let n = d.blocks[0].text.len();
        d.blocks[0].splice(0, n, "<div>");
    });
}

#[test]
fn a_block_appended_after_an_unclosed_fence_stays_outside_the_code() {
    edited("```\ncode", |d| d.blocks.push(Block::paragraph("added")));
}

#[test]
fn the_fix_keeps_a_one_word_edit_at_one_line() {
    let src = "# T\n\nfirst words here\n\nsecond\n\n- a\n- b\n";
    let mut d = import_markdown(src);
    d.blocks[1].splice(0, 5, "other");
    let out = export(&d);
    assert_eq!(out, "# T\n\nother words here\n\nsecond\n\n- a\n- b\n");
}

#[test]
fn the_hash_covers_the_source_text_also_after_json() {
    let mut d = import_markdown("Title\n=====\n\nreal\n");
    d.blocks[1].attrs.source.as_mut().unwrap().text = "TAMPERED\n".into();
    assert!(!export(&d).contains("TAMPERED"));
    let mut d = import_markdown("Title\n=====\n\nreal\n");
    let json = serde_json::to_string(&d)
        .unwrap()
        .replace("real\\n\",\"prev", "TAMPERED\\n\",\"prev");
    d = serde_json::from_str(&json).unwrap();
    assert!(!export(&d).contains("TAMPERED"));
}

#[test]
fn a_layer_mark_does_not_invalidate_the_source() {
    let mut d = import_markdown("Title\n=====\n");
    d.blocks[0].add_mark(Mark::new(
        MarkKind::Passage,
        0,
        5,
        MarkValue::Name("p".into()),
    ));
    assert_eq!(export(&d), "Title\n=====\n");
}

#[test]
fn the_fingerprint_covers_containers_and_hints() {
    let src = "- a\n- b\n";
    let mut d = import_markdown(src);
    d.blocks[0].attrs.containers[0] = match &d.blocks[0].attrs.containers[0] {
        Container::Item { inst, list, .. } => Container::Item {
            inst: *inst,
            list: *list,
            number: None,
            marker: '*',
            tight: true,
            task: None,
        },
        c => c.clone(),
    };
    assert_ne!(
        export(&d),
        src,
        "container changed, source must not be used"
    );
    let src = "a\n\n---\n\nb\n";
    let mut d = import_markdown(src);
    d.blocks[1].attrs.extra.insert("rule".into(), "***".into());
    assert_eq!(
        export(&d),
        "a\n\n***\n\nb\n",
        "hint changed, source must not be used"
    );
}
