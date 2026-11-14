//! The pictures of the wiki window over a real wiki with a real source (Handbook with a subpage, Notes): reading with the
//! outline, the page tree with a subpage, the dialogs (new page, subpage, rename, delete) and the editor, in Confluence and
//! Notion, German and English. The pictures are kept in the folder named by `FS_WIKI_SHOTS` (else in a temporary one).
//!
//! Pattern: Observation on the picture of the real window (`AppWidget::picture`); the asserts are the facts the pictures show
//! (the title of the page, the dialog open or closed), the look is judged on the pictures themselves.

// Linux only: the picture of the window needs a Vulkan adapter (software Vulkan in the Linux CI) and the fonts of that CI.
#![cfg(target_os = "linux")]

mod admin_common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::overlay_state::overlay_stack;
use fs_components_render::{Canvas, draw_to_png};
use fs_components_workbenches::{Reader, ReaderWidget, register_fonts};
use fs_wiki_app::WikiSource;
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{
    PAGE_DELETE, PAGE_NEW, PAGE_RENAME, PAGE_SUB, Setup, window_with_modal,
};
use fs_wiki_app::source::PageMaker;
use fs_wiki_app::ui_texts::UiTexts;
use fs_wiki_core::Texts;

const HANDBOOK: &str = "# Handbook\n\nWelcome to the handbook. It has several parts and says where to start.\n\n## Getting started\n\nInstall the program, start the server, open the window.\n\n- one list item\n- another list item\n- a third one\n\n### Details\n\nA little more about the first steps, in a paragraph of its own.\n\n## Reference\n\nThe closing words of the handbook.\n";

fn shots() -> (PathBuf, Option<tempfile::TempDir>) {
    if let Some(dir) = std::env::var_os("FS_WIKI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        return (PathBuf::from(dir), None);
    }
    let t = tempfile::tempdir().unwrap();
    (t.path().to_path_buf(), Some(t))
}

fn wait_menu(app: &fs_components_app::AppWidget, id: &str) {
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(5) {
        let on = app.with_shell(|s| {
            s.menus()
                .iter()
                .flat_map(|m| m.entries.iter())
                .any(|e| e.id == id && !e.disabled)
        });
        if on {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The window as it is composed on the screen: the program's element and Blinc's overlay stack (the dialogs) above it.
fn composed(app: &fs_components_app::AppWidget, w: f32, h: f32) -> Div {
    let layer = {
        let stack = overlay_stack();
        let mut stack = stack.lock().unwrap();
        stack.set_viewport_with_scale(w, h, 1.0);
        // Settle the entering motion of a dialog: the picture shows it at rest.
        stack.update(u64::MAX / 2);
        stack.build_overlay_layer()
    };
    div()
        .w(w)
        .h(h)
        .relative()
        .child(app.build(w, h))
        .child(layer)
}

fn picture(app: &fs_components_app::AppWidget, dir: &std::path::Path, name: &str) {
    let file = dir.join(format!("{name}.png"));
    let canvas = Canvas::new(1100, 700, app.styles());
    draw_to_png(&canvas, |w, h| composed(app, w, h), || {}, &file).expect("picture");
    assert!(std::fs::metadata(&file).unwrap().len() > 2000, "{name}");
}

#[test]
fn the_wiki_window_in_both_frontends_and_languages() {
    let (dir, _keep) = shots();
    register_fonts();
    for (frontend, short) in [
        (Frontend::Confluence, "confluence"),
        (Frontend::Notion, "notion"),
    ] {
        for german in [false, true] {
            let lang = if german { "de" } else { "en" };
            let w = admin_common::wiki();
            let rt = &w.rt;
            let texts: Arc<dyn Texts> = w.texts.clone();
            for (path, text) in [
                ("handbook.md", HANDBOOK),
                (
                    "handbook/chapter-one.md",
                    "# Chapter one\n\nThe first chapter of the handbook.\n",
                ),
                ("notes.md", "# Notes\n\nA short note.\n"),
            ] {
                rt.block_on(texts.create(path, text)).unwrap();
            }
            let mut source = WikiSource::new(Arc::clone(&texts), rt.handle().clone(), vec![]);
            let bare = source.bare_switch();
            if frontend.has_own_head() {
                source = source.bare_pages();
            }
            let source = Arc::new(source);
            let maker: Arc<dyn PageMaker> = source.clone();
            let t = UiTexts::new(german);
            let reader = ReaderWidget::new(source, t.reader_labels(), None);
            reader.with_reader(Reader::refresh_entries);
            let setup = Setup {
                labels: t.wiki_labels(),
                german,
                bare,
                settings: None,
                pages: Some(maker),
            };
            let (app, modal) =
                window_with_modal("FreeSynergy Wiki", reader.clone(), frontend, setup).unwrap();
            let name = |what: &str| format!("{short}-{lang}-{what}");
            // Reading: the outline (Confluence), the title once, the way to the page.
            reader.with_reader(|r| r.open("handbook.md"));
            assert_eq!(reader.with_reader(|r| r.page_title()), "Handbook");
            wait_menu(&app, PAGE_SUB);
            picture(&app, &dir, &name("1-lesen"));
            reader.with_reader(|r| r.open("handbook/chapter-one.md"));
            picture(&app, &dir, &name("2-baum-unterseite"));
            reader.with_reader(|r| r.open("handbook.md"));
            for (id, what) in [
                (PAGE_NEW, "3-neue-seite"),
                (PAGE_SUB, "4-unterseite"),
                (PAGE_RENAME, "5-umbenennen"),
                (PAGE_DELETE, "7-loeschen"),
            ] {
                assert!(app.choose(id), "{id}");
                picture(&app, &dir, &name(what));
                if id == PAGE_NEW {
                    // A taken name stays in the dialog, with the reason under it.
                    assert!(modal.submit("Notes"));
                    picture(&app, &dir, &name("3b-name-vergeben"));
                }
                assert!(modal.no());
            }
            reader.with_reader(Reader::begin_edit);
            picture(&app, &dir, &name("6-bearbeiten"));
            if german {
                // The states after the changes: renamed (with its subpage), deleted, and what the trash shows.
                reader.with_reader(|r| {
                    r.cancel_edit();
                    r.cancel_edit();
                    r.open("handbook.md");
                });
                wait_menu(&app, PAGE_SUB);
                assert!(app.choose(PAGE_RENAME));
                assert!(modal.submit("Manual"));
                picture(&app, &dir, &name("8-nach-umbenennen"));
                reader.with_reader(|r| r.open("notes.md"));
                wait_menu(&app, PAGE_DELETE);
                assert!(app.choose(PAGE_DELETE));
                assert!(modal.yes());
                picture(&app, &dir, &name("9-nach-loeschen"));
                if short == "confluence" {
                    let registry = rt.block_on(w.texts.types()).unwrap();
                    let admin =
                        fs_wiki_app::admin::shell_in(&registry, &w.ctx, "source", true).unwrap();
                    assert!(admin.select("TrashEntry"));
                    picture(&admin, &dir, "admin-de-10-papierkorb");
                }
            }
        }
    }
}
