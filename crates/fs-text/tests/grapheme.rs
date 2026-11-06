//! Grapheme-safe offsets: marks and edits never split a user-perceived character.

use fs_text::*;
mod common;
use common::block_with;

const THUMB: &str = "👍🏽"; // emoji + skin tone (2 chars)
const FAMILY: &str = "👨\u{200d}👩\u{200d}👧"; // ZWJ sequence (5 chars)
const ACUTE: &str = "e\u{301}"; // e + combining acute (2 chars)
const FLAG: &str = "🇩🇪"; // two regional indicators

fn text_of(d: &Document) -> String {
    d.blocks[0].text.to_string()
}

fn shown(d: &Document, m: &Mark) -> String {
    d.blocks[0].text.slice(m.start..m.end)
}

#[test]
fn marking_half_an_emoji_marks_the_whole_emoji() {
    for (cluster, n) in [(THUMB, 2), (FAMILY, 5), (ACUTE, 2), (FLAG, 2)] {
        let (mut d, id) = block_with(&format!("a{cluster}b"), vec![]);
        // from the middle of the cluster to its end
        d.apply(&Op::AddMark {
            block: id,
            mark: Mark::new(MarkKind::Strong, 2, 1 + n, MarkValue::None),
        })
        .unwrap();
        let m = &d.blocks[0].marks.as_slice()[0];
        assert_eq!((m.start, m.end), (1, 1 + n), "{cluster:?}");
        assert_eq!(shown(&d, m), cluster);
    }
}

#[test]
fn mark_touching_two_clusters_covers_both() {
    let (mut d, id) = block_with(&format!("{ACUTE}{THUMB}x"), vec![]);
    d.apply(&Op::AddMark {
        block: id,
        mark: Mark::new(MarkKind::Passage, 1, 3, MarkValue::Name("p".into())),
    })
    .unwrap();
    let m = &d.blocks[0].marks.as_slice()[0];
    assert_eq!(shown(&d, m), format!("{ACUTE}{THUMB}"));
}

#[test]
fn point_marks_snap_to_the_nearest_boundary() {
    let mut b = Block::paragraph(&format!("a{FAMILY}b"));
    b.add_mark(Mark::point(MarkKind::Variable, 2, MarkValue::None));
    b.add_mark(Mark::point(
        MarkKind::FootnoteRef,
        5,
        MarkValue::Name("n".into()),
    ));
    let at: Vec<usize> = b.marks.iter().map(|m| m.start).collect();
    assert_eq!(at, [1, 6]);
}

#[test]
fn inserting_inside_a_cluster_lands_on_a_boundary() {
    let (mut d, id) = block_with(&format!("a{THUMB}b"), vec![]);
    let op = d.insert_op(id, 2, "!").unwrap();
    d.apply(&op).unwrap();
    assert_eq!(text_of(&d), format!("a!{THUMB}b"));
}

#[test]
fn deleting_half_a_cluster_deletes_all_of_it() {
    let (mut d, id) = block_with(&format!("a{FAMILY}b"), vec![]);
    let op = d.delete_op(id, 2, 4).unwrap();
    d.apply(&op).unwrap();
    assert_eq!(text_of(&d), "ab");
    let (mut d, id) = block_with(&format!("x{ACUTE}y"), vec![]);
    let op = d.delete_op(id, 2, 3).unwrap(); // only the combining mark
    d.apply(&op).unwrap();
    assert_eq!(text_of(&d), "xy");
}

#[test]
fn user_replace_takes_whole_clusters_but_plain_ops_stay_exact() {
    let (mut d, id) = block_with(&format!("a{THUMB}b"), vec![]);
    // user edit: replacing the skin tone replaces the whole emoji
    let op = d.edit_op(id, 2, 3, "X").unwrap();
    assert!(matches!(
        op,
        Op::Edit {
            start: 1,
            end: 3,
            ..
        }
    ));
    // plain op (external import, undo): exact offsets, text reproduced exactly
    let exact = Op::Edit {
        block: id,
        start: 2,
        end: 3,
        insert: "\u{1F3FF}".into(),
        restore: None,
    };
    d.apply(&exact).unwrap();
    assert_eq!(text_of(&d), "a👍🏿b");
    d.apply(&op).unwrap();
    assert_eq!(text_of(&d), "aXb");
}

#[test]
fn bad_ranges_are_errors() {
    let (d, id) = block_with("abc", vec![]);
    assert_eq!(
        d.edit_op(id, 2, 9, ""),
        Err(ApplyError::OutOfRange(2, 9, 3))
    );
}

#[test]
fn snapped_edits_undo_exactly() {
    let (mut d, id) = block_with(
        &format!("a{FAMILY}b"),
        vec![Mark::new(
            MarkKind::Passage,
            1,
            6,
            MarkValue::Name("f".into()),
        )],
    );
    let orig = d.clone();
    let op = d.delete_op(id, 2, 4).unwrap();
    let inv = d.apply(&op).unwrap();
    assert!(
        d.blocks[0].marks.is_empty(),
        "the passage covered only the deleted cluster"
    );
    d.apply(&inv).unwrap();
    assert_eq!(d, orig);
}

#[test]
fn crlf_is_one_unit() {
    let (mut d, id) = block_with("a\r\nb", vec![]);
    let op = d.delete_op(id, 1, 2).unwrap();
    d.apply(&op).unwrap();
    assert_eq!(text_of(&d), "ab");
}

#[test]
fn plain_text_is_unchanged() {
    let (mut d, id) = block_with("hello", vec![]);
    d.apply(&Op::AddMark {
        block: id,
        mark: Mark::new(MarkKind::Strong, 1, 3, MarkValue::None),
    })
    .unwrap();
    let op = d.insert_op(id, 2, "XY").unwrap();
    d.apply(&op).unwrap();
    let m = &d.blocks[0].marks.as_slice()[0];
    assert_eq!(shown(&d, m), "eXYl");
}
