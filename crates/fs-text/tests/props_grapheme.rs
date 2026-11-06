//! Property tests: marks sit on grapheme boundaries; user edits keep clusters whole and undo.

use fs_text::*;
use proptest::prelude::*;
mod common;
use common::block_with;

const THUMB: &str = "👍🏽";
const FAMILY: &str = "👨\u{200d}👩\u{200d}👧";
const ACUTE: &str = "e\u{301}";
const FLAG: &str = "🇩🇪";

fn text_of(d: &Document) -> String {
    d.blocks[0].text.to_string()
}

fn alphabet() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            Just("a"),
            Just(THUMB),
            Just(FAMILY),
            Just(ACUTE),
            Just(FLAG),
            Just("\r\n"),
            Just("é")
        ],
        0..8,
    )
    .prop_map(|v| v.concat())
}

proptest! {
    #[test]
    fn marks_always_sit_on_boundaries(text in alphabet(), s in 0usize..30, n in 0usize..12) {
        let (mut d, id) = block_with(&text, vec![]);
        d.apply(&Op::AddMark { block: id, mark: Mark::new(MarkKind::Passage, s, s + n, MarkValue::Name("p".into())) }).unwrap();
        let t = &d.blocks[0].text;
        for m in d.blocks[0].marks.iter() {
            prop_assert!(t.is_boundary(m.start) && t.is_boundary(m.end), "{m:?} in {text:?}");
        }
    }

    #[test]
    fn user_edits_keep_clusters_whole_and_undo(text in alphabet(), a in 0usize..30, n in 0usize..12, ins in prop_oneof![Just(""), Just("x"), Just(THUMB)]) {
        let (mut d, id) = block_with(&text, vec![]);
        let orig = d.clone();
        let len = d.blocks[0].text.len();
        let (a, b) = (a.min(len), (a + n).min(len));
        let op = d.edit_op(id, a, b, ins).unwrap();
        let inv = d.apply(&op).unwrap();
        let (s, e) = orig.blocks[0].text.snap_range(a, b);
        let orig_text = text_of(&orig);
        let head: String = orig_text.chars().take(s).collect();
        let tail: String = orig_text.chars().skip(e).collect();
        prop_assert_eq!(text_of(&d), format!("{head}{ins}{tail}"));
        d.apply(&inv).unwrap();
        prop_assert_eq!(d, orig);
    }
}
