//! Ticking a checkbox on the real wiki: one ordinary change of the stored text, a conflict is refused, every other byte stays
//! (finding W4 of 2026-11-14). The reader's click reaches `WikiSource::tick` once fs-components 0.0.19 is the pin
//! (`DocumentSource::set_task`); this test calls it directly.
//!
//! Pattern: Integration test over the public API, no window.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use fs_components_workbenches::DocumentSource;
use fs_wiki_app::WikiSource;

const TEXT: &str = "# Tasks\n\n| a | b |\n| --- | --- |\n| x | [ ] |\n\n- [ ] one\n- [x] two\n";

#[test]
fn a_tick_is_one_change_of_the_stored_text_and_nothing_else_changes() {
    let (rt, fixture) = common::wiki();
    let wiki = fixture.wiki().texts();
    rt.block_on(wiki.create("tasks.md", TEXT)).unwrap();
    let source = Arc::new(WikiSource::new(
        Arc::clone(&wiki),
        rt.handle().clone(),
        vec![],
    ));
    let text = |s: &WikiSource| s.source_text("tasks.md").unwrap().text;
    let before = source.source_text("tasks.md").unwrap();
    assert_eq!(before.text, TEXT);
    // Box 1 is the first list task.
    source
        .tick("tasks.md", 1, true, before.revision.as_deref())
        .unwrap();
    assert_eq!(text(&source), TEXT.replace("- [ ] one", "- [x] one"));
    // The table cell is box 0; the revision of the first read is stale now: refused, nothing changes.
    let stale = source.tick("tasks.md", 0, true, before.revision.as_deref());
    assert!(stale.is_err(), "a conflict is refused");
    let now = source.source_text("tasks.md").unwrap();
    source
        .tick("tasks.md", 0, true, now.revision.as_deref())
        .unwrap();
    assert_eq!(
        text(&source),
        TEXT.replace("- [ ] one", "- [x] one")
            .replace("| x | [ ] |", "| x | [x] |")
    );
    // A box that is not there is refused.
    assert!(source.tick("tasks.md", 9, true, None).is_err());
}
