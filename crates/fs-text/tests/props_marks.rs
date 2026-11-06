//! Property tests: marks stay valid and undo is exact under random edits.

use fs_text::*;
use proptest::prelude::*;
mod common;
use common::block_with;

#[derive(Clone, Debug)]
enum E {
    Ins(usize, String),
    Del(usize, usize),
}

fn edit_strategy() -> impl Strategy<Value = Vec<E>> {
    proptest::collection::vec(
        prop_oneof![
            (0usize..40, "[a-zé ]{0,5}").prop_map(|(a, s)| E::Ins(a, s)),
            (0usize..40, 0usize..8).prop_map(|(a, n)| E::Del(a, a + n))
        ],
        0..25,
    )
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
                _ => Mark::new(
                    MarkKind::Link,
                    s,
                    e,
                    MarkValue::Link(LinkValue::new("x", LinkStyle::Inline)),
                ),
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
}
