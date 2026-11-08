//! Undo / redo and retention.

use fs_text::*;
mod common;
use common::block_with;

#[test]
fn history_undo_redo_and_retention() {
    let (mut d, id) = block_with(
        "hello world",
        vec![Mark::new(
            MarkKind::Passage,
            6,
            11,
            MarkValue::Name("w".into()),
        )],
    );
    let orig = d.clone();
    let mut h = History::new(Retention {
        max_age: None,
        max_steps: Some(3),
    });
    h.apply(&mut d, vec![Op::delete_text(id, 5, 11)], "cut", 100)
        .unwrap();
    assert_eq!(d.blocks[0].text.to_string(), "hello");
    assert!(h.undo(&mut d).unwrap());
    assert_eq!(d, orig, "undo restores text and the dead mark");
    assert!(h.redo(&mut d).unwrap());
    assert_eq!(d.blocks[0].text.to_string(), "hello");
    assert!(h.undo(&mut d).unwrap());
    // retention by steps: 5 steps, only 3 undoable, pending ops kept for sync
    for i in 0..5 {
        h.apply(
            &mut d,
            vec![Op::insert_text(id, 0, &i.to_string())],
            "type",
            200 + i,
        )
        .unwrap();
    }
    assert_eq!(h.len(), 3);
    assert_eq!(
        h.pending_sync().len(),
        2,
        "compacted steps stay until synced"
    );
    let current = d.clone();
    while h.undo(&mut d).unwrap() {}
    assert_ne!(d, current);
    while h.redo(&mut d).unwrap() {}
    assert_eq!(d, current);
    h.mark_synced();
    assert_eq!(h.pending_sync(), []);
    // retention by age
    let mut h = History::new(Retention {
        max_age: Some(60),
        max_steps: None,
    });
    h.apply(&mut d, vec![Op::insert_text(id, 0, "a")], "old", 0)
        .unwrap();
    h.apply(&mut d, vec![Op::insert_text(id, 0, "b")], "new", 1000)
        .unwrap();
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
    let r = h.apply(
        &mut d,
        vec![Op::insert_text(id, 0, "x"), Op::delete_text(id, 0, 99)],
        "bad",
        0,
    );
    assert!(r.is_err());
    assert_eq!(d, before);
}
