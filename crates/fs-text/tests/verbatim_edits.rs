//! Moved, split, merged, removed blocks and rewritten links against kept source texts.

use fs_text::*;

fn export(d: &Document) -> String {
    export_markdown(d, ExportOptions::default())
}

#[test]
fn moved_split_and_merged_blocks_are_written_new() {
    let src = "one\n\ntwo\n\nthree\n";
    // moved: the block keeps its content, but not its place
    let mut d = import_markdown(src);
    let b = d.blocks.remove(2);
    d.blocks.insert(0, b);
    let out = export(&d);
    assert_eq!(out, "three\n\none\n\ntwo\n");
    assert!(d.same_content(&import_markdown(&out)));
    // a moved block must not glue to its new neighbour: "two" + "three" would read as one paragraph
    let src = "# H\ntext\n\nlast\n";
    let mut d = import_markdown(src);
    let last = d.blocks.remove(2);
    d.blocks.insert(1, last);
    let out = export(&d);
    let texts = |d: &Document| {
        d.blocks
            .iter()
            .map(|b| b.text.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(&d), texts(&import_markdown(&out)), "{out:?}");
    // split: one block becomes two
    let mut d = import_markdown("alpha beta\n\nz\n");
    let mut second = d.blocks[0].clone();
    second.id = Block::paragraph("").id;
    second.attrs.source = None;
    second.delete(0, 6);
    d.blocks[0].delete(5, 10);
    d.blocks.insert(1, second);
    let out = export(&d);
    assert_eq!(out, "alpha\n\nbeta\n\nz\n");
    // merged: two blocks become one
    let mut d = import_markdown("alpha\n\nbeta\n\nz\n");
    let gone = d.blocks.remove(1);
    let n = d.blocks[0].text.len();
    d.blocks[0].insert(n, &format!(" {}", gone.text));
    let out = export(&d);
    assert_eq!(out, "alpha beta\n\nz\n");
    assert!(d.same_content(&import_markdown(&out)));
}

#[test]
fn a_removed_block_takes_its_text_with_it_and_the_rest_is_kept() {
    let mut d = import_markdown("# H\n\none\n\n\n\ntwo\n\nthree  \nx\n");
    d.blocks.remove(1);
    let out = export(&d);
    assert!(out.contains("two") && out.contains("three  \nx"), "{out:?}");
    assert!(!out.contains("one"));
}

#[test]
fn rewritten_links_are_changes() {
    let src = "see [a](old/page.md) here\n\nother\n";
    let mut d = import_markdown(src);
    let mut docs = vec![d.clone()];
    let ops = rewrite_links(&mut docs, "old", "new");
    assert!(!ops.is_empty() || docs[0] != d);
    d = docs.remove(0);
    let out = export(&d);
    assert!(
        out.contains("new/page.md") && !out.contains("old/page.md"),
        "{out:?}"
    );
    assert!(out.ends_with("\n\nother\n"));
}
