mod common;
use fs_wiki_core::Texts;
#[tokio::test]
async fn dbg() {
    let w = common::wiki().await;
    let q = w
        .create(
            "q.md",
            "Use [r].\n\n[r]: t/p.md\n\n[r]: t/p.md#other\n\n[s]: t/a.md\n[s]: t/b.md\n",
        )
        .await
        .unwrap();
    for b in &q.document.blocks {
        eprintln!(
            "BLK {:?} {:?} {:?}",
            b.attrs.kind,
            b.attrs.extra,
            b.text.to_string()
        );
    }
}
