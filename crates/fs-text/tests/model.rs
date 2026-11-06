//! Anchors and serde round trip of the model.

use fs_text::*;
mod common;

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
