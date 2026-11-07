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
fn the_binary_without_a_server_fails_with_that_message_and_creates_no_store() {
    let dir = temp_dir("sv-bin");
    let (socket, docs) = (dir.join("none.sock"), dir.join("docs"));
    std::fs::create_dir_all(&docs).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_fs-wiki-app"))
        .args(["--socket", &socket.display().to_string()])
        .args(["--docs", &docs.display().to_string()])
        .args(["--origins", &dir.join("o").display().to_string()])
        .args(["--drafts", &dir.join("drafts").display().to_string()])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        err.contains("cannot reach the wiki server") && err.contains("fs-wiki-server"),
        "{err}"
    );
    assert!(
        !dir.join("o").exists(),
        "nothing was started without a server"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
