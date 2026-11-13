//! The normal start (what the desktop file runs: `fs-wiki`, no arguments for the window) shows the remembered frontend.
//!
//! Pattern: Example-based tests of the real binary, in a temporary HOME and data folder; the one added option is
//! `--snapshot` (a picture instead of the window), which reports the frontend it drew. Nothing of the real user is touched.
//! A start also starts a server (`fs-wiki serve --lease`); `Home` is the guard that no such server outlives its test.

#![cfg(feature = "gui")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The temporary folder of one test. Dropping it ends every server that was started on a socket inside it (also after a
/// failed assertion), waits until they are gone and removes the folder.
struct Home(PathBuf);

impl Home {
    fn new(name: &str) -> Home {
        let dir = std::env::temp_dir().join(format!("fs-wiki-start-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Home(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        for pid in servers_in(&self.0) {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
        wait_until_gone(&self.0, Duration::from_secs(10));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The processes `fs-wiki serve ...` that have a word of their command line inside `dir` (Linux: `/proc`).
fn servers_in(dir: &Path) -> Vec<u32> {
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let dir = dir.to_string_lossy().into_owned();
    procs
        .filter_map(|p| {
            let p = p.ok()?;
            let pid: u32 = p.file_name().to_str()?.parse().ok()?;
            let line = std::fs::read(p.path().join("cmdline")).ok()?;
            let words: Vec<String> = line
                .split(|b| *b == 0)
                .map(|w| String::from_utf8_lossy(w).into_owned())
                .collect();
            let serves = words.iter().any(|w| w == "serve");
            (serves && words.iter().any(|w| w.starts_with(&dir))).then_some(pid)
        })
        .collect()
}

fn wait_until_gone(dir: &Path, within: Duration) {
    let began = Instant::now();
    while !servers_in(dir).is_empty() && began.elapsed() < within {
        std::thread::sleep(Duration::from_millis(50));
    }
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
        // The server the start makes looks at its lease every 100 ms instead of every 2 s.
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

fn settings_file(home: &Path) -> PathBuf {
    home.join("data/freesynergy/fs-wiki/frontend")
}

fn drawn(said: &str) -> &str {
    let (_, shown) = said
        .lines()
        .find_map(|l| l.split_once(", frontend "))
        .unwrap_or_else(|| panic!("no frontend in: {said}"));
    shown.trim()
}

#[test]
fn the_first_start_shows_confluence_then_the_remembered_frontend_and_a_broken_file_means_the_default() {
    let home = Home::new("frontends");
    let home = home.path();
    assert_eq!(drawn(&start(home, &[])), "a", "first start: Confluence");
    // Nothing was written by merely starting.
    assert!(!settings_file(home).exists());
    for (word, id) in [("b\n", "b"), ("classic\n", "classic"), ("a\n", "a")] {
        std::fs::create_dir_all(settings_file(home).parent().unwrap()).unwrap();
        std::fs::write(settings_file(home), word).unwrap();
        assert_eq!(drawn(&start(home, &[])), id, "remembered {id}");
    }
    std::fs::write(settings_file(home), "no such frontend").unwrap();
    assert_eq!(drawn(&start(home, &[])), "a", "broken file: default");
}

#[test]
fn the_choice_of_an_earlier_version_is_found_under_its_old_name_and_kept_under_the_new() {
    let home = Home::new("migrate");
    let home = home.path();
    let folder = settings_file(home).parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("face"), "b\n").unwrap();
    assert_eq!(drawn(&start(home, &[])), "b");
    assert_eq!(std::fs::read_to_string(settings_file(home)).unwrap(), "b\n");
    assert!(folder.join("face").is_file(), "the old file stays");
}

#[test]
fn an_override_on_the_command_line_wins_and_writes_nothing() {
    let home = Home::new("override");
    let home = home.path();
    std::fs::create_dir_all(settings_file(home).parent().unwrap()).unwrap();
    std::fs::write(settings_file(home), "b\n").unwrap();
    assert_eq!(drawn(&start(home, &["--frontend", "classic"])), "classic");
    assert_eq!(std::fs::read_to_string(settings_file(home)).unwrap(), "b\n");
}

#[cfg(target_os = "linux")]
#[test]
fn the_server_a_start_made_ends_with_the_window_even_when_its_folder_is_gone() {
    let home = Home::new("ends");
    start(home.path(), &[]);
    // The window is gone; so is the folder with the socket and the lease, as in every test that cleans up after itself.
    std::fs::remove_dir_all(home.path()).unwrap();
    wait_until_gone(home.path(), Duration::from_secs(30));
    assert!(
        servers_in(home.path()).is_empty(),
        "a server of this test is still running"
    );
}
