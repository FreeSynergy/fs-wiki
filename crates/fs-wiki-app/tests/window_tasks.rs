//! The window over the real wiki: a click on a checkbox in the reading view stores one ordinary change (it is in the history),
//! without the right to change the page the boxes are locked and nothing is stored; the page count and the block menu speak the
//! language of the window; the table of the user test (centred column) does not break "Beschreibung" (finding W1/W4 of 2026-11-14).
//!
//! Pattern: Integration test on the real wiki with a mouse click through the router and hit test of a window (no window opens).
//! Pictures of the window (`AppWidget::picture`) are kept in the folder named by `FS_WIKI_SHOTS`.

#![cfg(target_os = "linux")]

mod page_rig;

use fs_components::blinc::layout::event_router::{EventRouter, MouseButton};
use fs_components::blinc::layout::prelude::RenderTree;
use fs_components_workbenches::{DocumentSource, WikiLabels};
use fs_wiki_app::WikiSource;
use fs_wiki_app::ui_texts::UiTexts;
use fs_wiki_core::Texts;
use page_rig::rig;

const TEXT: &str = "# Tasks\n\n| a | b |\n| --- | --- |\n| x | [ ] |\n\n- [ ] one\n- [x] two\n";

/// Press and release the left button in the middle of the element with this id, then draw what that asked for.
fn click(tree: &mut RenderTree, id: &str) {
    let node = tree
        .query_by_id(id)
        .unwrap_or_else(|| panic!("no element {id}"));
    let b = tree.get_absolute_bounds(node).expect("bounds");
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    let mut router = EventRouter::new();
    for events in [
        router.on_mouse_move(tree, x, y),
        router.on_mouse_down(tree, x, y, MouseButton::Left),
        router.on_mouse_up(tree, x, y, MouseButton::Left),
    ] {
        for (n, e) in events {
            tree.dispatch_event(n, e, x, y);
        }
    }
    tree.process_pending_subtree_rebuilds();
    tree.compute_layout(1000.0, 700.0);
}

/// The picture of the window, kept in the folder named by `FS_WIKI_SHOTS`.
fn shot(r: &page_rig::Rig, name: &str) {
    if let Some(dir) = std::env::var_os("FS_WIKI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        let _ = r
            .app
            .picture(1000, 520, &std::path::Path::new(&dir).join(name));
    }
}

fn revisions(r: &page_rig::Rig, path: &str) -> usize {
    r.rt.block_on(async {
        let entry = r.wiki.get_by_path(path).await.unwrap().unwrap();
        r.wiki
            .history(entry.id, None, 100)
            .await
            .unwrap()
            .items
            .len()
    })
}

/// One test at a time may set Blinc up (process-wide state); the picture of the window sets it up like a window does.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn opened_tree(r: &page_rig::Rig) -> RenderTree {
    let warm = std::env::temp_dir().join(format!("fs-wiki-tasks-{}.png", std::process::id()));
    let _ = r.app.picture(1000, 700, &warm);
    let _ = std::fs::remove_file(&warm);
    fs_components::blinc::core::reactive::set_stateful_deps_notifier(|ids| {
        fs_components::blinc::layout::check_stateful_deps(ids);
    });
    let mut tree = RenderTree::from_element(&r.reader.build(1000.0, 700.0));
    tree.compute_layout(1000.0, 700.0);
    tree
}

#[test]
fn a_click_on_a_checkbox_stores_one_change_and_only_the_box_changes() {
    let _g = serial();
    let r = rig(true);
    r.make("tasks.md", TEXT);
    r.open("tasks.md");
    let before = revisions(&r, "tasks.md");
    let mut tree = opened_tree(&r);
    shot(&r, "wiki-0012-aufgaben-vorher.png");
    // Box 0 is the table cell, boxes 1 and 2 are the list tasks.
    click(&mut tree, "fs-tv-task-1");
    shot(&r, "wiki-0012-aufgaben-nachher.png");
    assert_eq!(r.text("tasks.md"), TEXT.replace("- [ ] one", "- [x] one"));
    assert_eq!(
        revisions(&r, "tasks.md"),
        before + 1,
        "one change in the history"
    );
    click(&mut tree, "fs-tv-task-0");
    assert_eq!(
        r.text("tasks.md"),
        TEXT.replace("- [ ] one", "- [x] one")
            .replace("| x | [ ] |", "| x | [x] |")
    );
    assert_eq!(revisions(&r, "tasks.md"), before + 2);
    // Ticking again is an ordinary change the other way round.
    click(&mut tree, "fs-tv-task-1");
    assert_eq!(
        r.text("tasks.md"),
        TEXT.replace("| x | [ ] |", "| x | [x] |")
    );
}

#[test]
fn without_the_right_to_change_the_page_the_boxes_are_locked_and_nothing_is_stored() {
    let _g = serial();
    let r = rig(false);
    let _ = opened_tree(&r);
    let (rt, fixture) = page_rig::common_wiki();
    let wiki = fixture.wiki().texts();
    rt.block_on(wiki.create("tasks.md", TEXT)).unwrap();
    let source =
        std::sync::Arc::new(WikiSource::new(wiki.clone(), rt.handle().clone(), vec![]).read_only());
    assert!(
        !source.can_tick("tasks.md"),
        "a read-only wiki locks the boxes"
    );
    let reader = fs_components_workbenches::ReaderWidget::new(
        source,
        UiTexts::new(false).reader_labels(),
        None,
    );
    reader.with_reader(|r| r.open("tasks.md"));
    let before = rt.block_on(async {
        let e = wiki.get_by_path("tasks.md").await.unwrap().unwrap();
        wiki.history(e.id, None, 100).await.unwrap().items.len()
    });
    let mut tree = RenderTree::from_element(&reader.build(1000.0, 700.0));
    tree.compute_layout(1000.0, 700.0);
    assert!(
        tree.query_by_id("fs-tv-task-1").is_some(),
        "the state is drawn"
    );
    click(&mut tree, "fs-tv-task-1");
    let after = rt.block_on(async {
        let e = wiki.get_by_path("tasks.md").await.unwrap().unwrap();
        wiki.history(e.id, None, 100).await.unwrap().items.len()
    });
    assert_eq!(before, after, "nothing was stored");
}

#[test]
fn the_page_count_and_the_block_menu_speak_the_language_of_the_window() {
    let (de, en) = (UiTexts::new(true), UiTexts::new(false));
    let (wde, wen): (WikiLabels, WikiLabels) = (de.wiki_labels(), en.wiki_labels());
    assert_eq!(
        (wde.pages_said(1), wde.pages_said(2)),
        ("1 Seite".into(), "2 Seiten".into())
    );
    assert_eq!(
        (wen.pages_said(1), wen.pages_said(2)),
        ("1 page".into(), "2 pages".into())
    );
    assert_eq!(wde.slash.headings[0], "Überschrift 1");
    assert_eq!(wen.slash.headings[0], "Heading 1");
    assert_eq!(
        de.reader_labels().tasks_locked,
        "Du kannst diese Seite nicht ändern, deshalb sind ihre Kästchen gesperrt."
    );
}

/// The table of the user test with a centred column, in the real window (Confluence): header and rows are one line each.
const USER_TABLE: &str = "# Tabelle\n\n| Name | Test | Beschreibung |\n| --- | --- | :---: |\n| Kal | - [ ] Test? | Mal sehen |\n| ölasdkjf öldsakj fölksjd | ölasdkjfö alsdkfö asdlfk | öasdkjhföasldjfk aösdlfkjhaöslfj khaösdl fkh |\n";

#[test]
fn the_table_of_the_user_test_does_not_break_in_the_window() {
    let _g = serial();
    let r = rig(true);
    r.make("tabelle.md", USER_TABLE);
    r.open("tabelle.md");
    let dir =
        std::env::var_os("FS_WIKI_SHOTS").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("wiki-0012-tabelle-mitte.png");
    if r.app.picture(1400, 700, &file).is_err() {
        eprintln!("skipped: no graphics");
        return;
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&file).unwrap()));
    let mut reading = decoder.read_info().unwrap();
    let mut pixels = vec![0; reading.output_buffer_size().unwrap()];
    let info = reading.next_frame(&mut pixels).unwrap();
    let step = info.color_type.samples();
    let dark = |x: u32, y: u32| {
        let at = (y * info.width + x) as usize * step;
        pixels[at..at + 3].iter().all(|c| *c < 110)
    };
    // Lines of text in the table region of the middle column: header, two rows (the checkbox is no dark ink).
    let mut runs = 0;
    let mut inside = false;
    for y in 195..330 {
        let ink = (320..1040).any(|x| dark(x, y));
        if ink && !inside {
            runs += 1;
        }
        inside = ink;
    }
    assert_eq!(
        runs,
        3,
        "header and two rows are one line each; picture {}",
        file.display()
    );
}
