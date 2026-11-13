//! The work flows of a person at the wiki window, run on the real program: make a page, type into it and paste into it, save,
//! open it again, edit it, make a subpage (finding B-W of the test of 2026-11-13: "nothing can be made or pasted").
//!
//! Pattern: Example-based tests of the real binary in a temporary HOME. Each run of the program is one process with a picture
//! (`--snapshot`) and steps (`--step`, the interpreter of `mode_read_steps.rs`); the line `state: ...` it prints says what the
//! reader holds afterwards. The wiki lives in the server of the HOME, so a second run is "opening it again". `new` and
//! `subpage` are the entries of the menu Page. The text of a paste is given to the editor as the text area gives it (the whole
//! new text): the key chord Ctrl+V itself is Blinc's text area and needs a window with input, which these tests do not have.
//!
//! Linux only, as `start.rs` (Unix-socket server, `/proc`, software Vulkan).

#![cfg(all(feature = "gui", target_os = "linux"))]

use std::path::Path;
use std::process::Command;

mod common;

use common::Home;

/// One run of the program with these steps; returns what it said on stderr.
fn run(home: &Path, frontend: &str, extra: &[&str], steps: &[&str]) -> String {
    let socket = home.join("w.sock");
    let picture = home.join("shot.png");
    let mut command = Command::new(env!("CARGO_BIN_EXE_fs-wiki"));
    command
        .args(["--socket", &socket.display().to_string()])
        .args(["--snapshot", &picture.display().to_string()])
        .args(["--size", "600x400", "--lang", "en", "--frontend", frontend])
        .args(extra);
    for step in steps {
        command.args(["--step", step]);
    }
    let out = command
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_RUNTIME_DIR", home)
        .env("FS_LEASE_POLL_MS", "100")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{said}");
    assert!(picture.is_file(), "no picture: {said}");
    said
}

/// The part of the `state:` line between ` key=` and ` next=`.
fn part<'a>(said: &'a str, key: &str, next: &str) -> &'a str {
    let line = said
        .lines()
        .find(|l| l.starts_with("state: "))
        .unwrap_or_else(|| panic!("no state line in: {said}"));
    let from = line.find(&format!(" {key}=")).unwrap() + key.len() + 2;
    let to = line.find(&format!(" {next}=")).unwrap_or(line.len());
    &line[from..to]
}

fn stored(said: &str) -> &str {
    part(said, "stored", "notice")
}

fn pages(said: &str) -> &str {
    part(said, "pages", "zz")
}

#[test]
fn flow_1_a_new_page_is_made_in_every_frontend_without_any_option() {
    for frontend in ["a", "b", "classic"] {
        let home = Home::new(&format!("w1-{frontend}"));
        let said = run(home.path(), frontend, &[], &["new"]);
        assert_eq!(
            part(&said, "can_edit", "editing"),
            "true",
            "{frontend}: {said}"
        );
        assert_eq!(part(&said, "path", "can_edit"), "Some(\"new-page.md\")");
        assert_eq!(
            part(&said, "editing", "stored"),
            "Some(\"# New page\\n\")",
            "{said}"
        );
        assert!(pages(&said).contains("new-page.md = New page"), "{said}");
    }
}

#[test]
fn flow_1_a_read_only_window_makes_no_page_and_says_why() {
    let home = Home::new("w1-ro");
    let said = run(home.path(), "a", &["--read-only"], &["new"]);
    assert_eq!(part(&said, "can_edit", "editing"), "false", "{said}");
    assert!(said.contains("read only here"), "{said}");
    assert!(!pages(&said).contains("new-page"), "{said}");
}

#[test]
fn flows_2_and_3_typed_and_pasted_text_is_saved() {
    let home = Home::new("w23");
    // What is typed first, then a pasted block (several lines, umlauts, a tab, a quote): the text area reports the whole text.
    let typed = "# Notes\\nTyped by hand.\\n";
    let pasted = "\\nPasted:\\n\\n- Zeile ä ö ü ß €\\n- tab\there\\n\\n> a quote\\n";
    let text = format!("type:{typed}{pasted}");
    let said = run(home.path(), "a", &[], &["new:Notes", &text, "save"]);
    assert_eq!(
        part(&said, "editing", "stored"),
        "None",
        "saving ends the edit: {said}"
    );
    let held = stored(&said);
    for wanted in ["Typed by hand.", "Zeile ä ö ü ß €", "a quote", "Pasted:"] {
        assert!(held.contains(wanted), "{wanted} is not in {held}");
    }
    assert!(pages(&said).contains("notes.md = Notes"), "{said}");
}

#[test]
fn flows_4_and_5_a_saved_page_is_opened_again_and_edited() {
    let home = Home::new("w45");
    run(
        home.path(),
        "a",
        &[],
        &["new:Plan", "type:# Plan\\nfirst version\\n", "save"],
    );
    // A new process, a new window: the page is in the wiki.
    let again = run(home.path(), "b", &[], &["open:plan.md"]);
    assert!(stored(&again).contains("first version"), "{again}");
    // Edit it, save it, and look again.
    let edited = run(
        home.path(),
        "a",
        &[],
        &[
            "open:plan.md",
            "edit",
            "type:# Plan\\nsecond version\\n",
            "save",
        ],
    );
    assert!(stored(&edited).contains("second version"), "{edited}");
    let last = run(home.path(), "classic", &[], &["open:plan.md"]);
    assert!(stored(&last).contains("second version"), "{last}");
    assert!(!stored(&last).contains("first version"), "{last}");
}

#[test]
fn flow_6_a_subpage_lives_below_the_page_that_was_shown() {
    let home = Home::new("w6");
    let said = run(
        home.path(),
        "a",
        &[],
        &[
            "new:Handbook",
            "subpage:Chapter one",
            "type:# Chapter one\\ntext\\n",
            "save",
        ],
    );
    assert!(pages(&said).contains("handbook.md = Handbook"), "{said}");
    assert!(
        pages(&said).contains("handbook/chapter-one.md = Chapter one"),
        "{said}"
    );
    assert_eq!(
        part(&said, "path", "can_edit"),
        "Some(\"handbook/chapter-one.md\")"
    );
    // Without a page shown a subpage is refused (and says so); a second page with the same title is numbered.
    let numbered = run(home.path(), "b", &[], &["new:Handbook"]);
    assert!(
        pages(&numbered).contains("handbook-2.md = Handbook 2"),
        "{numbered}"
    );
    let refused = run(home.path(), "a", &[], &["subpage:Orphan"]);
    assert!(refused.contains("needs a page shown"), "{refused}");
    assert!(!pages(&refused).contains("orphan"), "{refused}");
}
