//! The normal start (what the desktop file runs: `fs-wiki`, no arguments for the window) shows the remembered face.
//!
//! Pattern: Example-based tests of the real binary, in a temporary HOME and data folder; the one added option is
//! `--snapshot` (a picture instead of the window), which reports the face it drew. Nothing of the real user is touched.

#![cfg(feature = "gui")]

use std::path::{Path, PathBuf};
use std::process::Command;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fs-wiki-start-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Start the program as the desktop file does (no mode option), only with a picture and a socket of the test.
fn start(home: &Path, extra: &[&str]) -> String {
    let socket = home.join("w.sock");
    let picture = home.join("shot.png");
    let out = Command::new(env!("CARGO_BIN_EXE_fs-wiki"))
        .args(["--socket", &socket.display().to_string()])
        .args(["--snapshot", &picture.display().to_string()])
        .args(["--size", "400x300"])
        .args(extra)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_RUNTIME_DIR", home)
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{said}");
    assert!(picture.is_file(), "no picture: {said}");
    said
}

fn face_file(home: &Path) -> PathBuf {
    home.join("data/freesynergy/fs-wiki/face")
}

fn drawn(said: &str) -> &str {
    let (_, face) = said
        .lines()
        .find_map(|l| l.split_once(", face "))
        .unwrap_or_else(|| panic!("no face in: {said}"));
    face.trim()
}

#[test]
fn the_first_start_shows_confluence_then_the_remembered_face_and_a_broken_file_means_the_default() {
    let home = temp("faces");
    assert_eq!(drawn(&start(&home, &[])), "a", "first start: Confluence");
    // Nothing was written by merely starting.
    assert!(!face_file(&home).exists());
    for (word, id) in [("b\n", "b"), ("classic\n", "classic"), ("a\n", "a")] {
        std::fs::create_dir_all(face_file(&home).parent().unwrap()).unwrap();
        std::fs::write(face_file(&home), word).unwrap();
        assert_eq!(drawn(&start(&home, &[])), id, "remembered {id}");
    }
    std::fs::write(face_file(&home), "no such face").unwrap();
    assert_eq!(drawn(&start(&home, &[])), "a", "broken file: default");
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn an_override_on_the_command_line_wins_and_writes_nothing() {
    let home = temp("override");
    std::fs::create_dir_all(face_file(&home).parent().unwrap()).unwrap();
    std::fs::write(face_file(&home), "b\n").unwrap();
    assert_eq!(drawn(&start(&home, &["--frontend", "classic"])), "classic");
    assert_eq!(std::fs::read_to_string(face_file(&home)).unwrap(), "b\n");
    std::fs::remove_dir_all(home).unwrap();
}
