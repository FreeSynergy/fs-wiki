//! Reaching the server: no fall-back on a store of the app, and a message that says what to do.
//!
//! Pattern: Integration tests against the public functions and the real binary.

mod common;

use std::path::Path;
use std::process::Command;

use common::temp_dir;
use fs_wiki_app::server::{connect, socket_path};

#[test]
fn a_given_socket_is_used_as_it_is() {
    let given = Path::new("/run/user/1/w.sock");
    assert_eq!(socket_path(Some(given)).unwrap(), given);
}

#[test]
fn nothing_listening_is_an_error_that_names_the_socket_and_how_to_start_the_server() {
    let dir = temp_dir("sv-none");
    let socket = dir.join("none.sock");
    let said = connect(&socket).err().expect("no server");
    assert_eq!(
        said.matches(&*socket.display().to_string()).count(),
        3,
        "{said}"
    );
    assert!(
        said.contains("fs-wiki-server") && said.contains("--socket"),
        "{said}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_binary_starts_the_server_of_its_version_on_a_given_socket_and_it_ends_with_the_window() {
    let dir = temp_dir("sv-bin");
    let socket = dir.join("own.sock");
    // `--check`: no window, but the same way to the server (handshake, start). The data goes to the temp folder.
    let out = Command::new(env!("CARGO_BIN_EXE_fs-wiki-app"))
        .args(["--check", "--socket", &socket.display().to_string()])
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_RUNTIME_DIR", &dir)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(said.contains("Page:"), "{said}");
    // Nobody holds the lease any more: the server ends by itself and takes its socket with it.
    let began = std::time::Instant::now();
    while socket.exists() && began.elapsed() < std::time::Duration::from_secs(30) {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    assert!(!socket.exists(), "the server it started did not end");
    std::fs::remove_dir_all(dir).unwrap();
}
