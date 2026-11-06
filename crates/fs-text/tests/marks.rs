//! Mark behaviour: expansion, points, merging, registry.

use fs_text::*;
mod common;
use common::{block_with, marks_of};

#[test]
fn expand_rules() {
    // strong expands after, not before; link expands neither way
    let (mut d, id) = block_with(
        "ab cd",
        vec![
            Mark::new(MarkKind::Strong, 3, 5, MarkValue::None),
            Mark::new(
                MarkKind::Link,
                0,
                2,
                MarkValue::Link(LinkValue::new("x", LinkStyle::Inline)),
            ),
        ],
    );
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
    let (mut d, id) = block_with(
        "hello world",
        vec![
            Mark::point(
                MarkKind::Variable,
                5,
                MarkValue::Variable {
                    name: "x".into(),
                    ty: None,
                },
            ),
            Mark::new(MarkKind::Passage, 6, 11, MarkValue::Name("w".into())),
        ],
    );
    // deleting only part of "world" keeps the passage
    d.apply(&Op::delete_text(id, 6, 8)).unwrap();
    assert_eq!(marks_of(&d).len(), 2);
    // deleting the rest removes it with the text
    let inv = d.apply(&Op::delete_text(id, 5, 9)).unwrap();
    assert!(d.blocks[0]
        .marks
        .iter()
        .all(|m| m.kind != MarkKind::Passage));
    // the point at 5 stays (boundary), whole-text deletion is undoable
    d.apply(&inv).unwrap();
    assert_eq!(d.blocks[0].text.to_string(), "hello rld");
    assert_eq!(marks_of(&d).len(), 2);
    // a point strictly inside a deleted range dies
    let (mut d, id) = block_with(
        "abcdef",
        vec![Mark::point(MarkKind::Variable, 3, MarkValue::None)],
    );
    d.apply(&Op::delete_text(id, 1, 5)).unwrap();
    assert!(marks_of(&d).is_empty());
}

#[test]
fn merge_and_unmark() {
    let mut s = MarkSet::new();
    s.add(Mark::new(MarkKind::Strong, 0, 3, MarkValue::None));
    s.add(Mark::new(MarkKind::Strong, 2, 6, MarkValue::None));
    assert_eq!(s.len(), 1);
    s.add(Mark::new(
        MarkKind::Link,
        0,
        3,
        MarkValue::Link(LinkValue::new("a", LinkStyle::Inline)),
    ));
    s.add(Mark::new(
        MarkKind::Link,
        1,
        4,
        MarkValue::Link(LinkValue::new("a", LinkStyle::Inline)),
    ));
    assert_eq!(s.len(), 3, "links never merge");
    s.unmark(MarkKind::Strong, 2, 4);
    let strongs: Vec<_> = s
        .iter()
        .filter(|m| m.kind == MarkKind::Strong)
        .map(|m| (m.start, m.end))
        .collect();
    assert_eq!(strongs, vec![(0, 2), (4, 6)]);
}

#[test]
fn registry_is_complete() {
    for s in REGISTRY {
        assert_eq!(s.kind.spec().name, s.name);
    }
    assert!(MarkKind::Variable.spec().point);
}
