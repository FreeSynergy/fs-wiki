//! A server a window started (`--lease`) lets go of what it inherited and ends when the folder of its lease is gone.
//!
//! Pattern: Example-based tests of the real binary; the guard kills the server whatever the test does (no orphan).

// Linux only: it looks at the descriptors of the server (`/proc`) and the server of a window is a Unix-socket server.
#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The server, killed when the guard goes (also when an assertion failed).
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Start the server as a window does (`--lease`), through a shell that first opens descriptor 9 without close-on-exec,
/// as a build tool does with its lock.
fn start(dir: &Path, lease: &Path) -> Server {
    let child = Command::new("sh")
        .args(["-c", "exec 9</dev/null; exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_fs-wiki-server"))
        .arg("--socket")
        .arg(dir.join("w.sock"))
        .arg("--data")
        .arg(dir.join("data"))
        .arg("--lease")
        .arg(lease)
        .env("FS_LEASE_POLL_MS", "100")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let server = Server(child);
    let began = Instant::now();
    while !dir.join("w.sock").exists() && began.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(dir.join("w.sock").exists(), "the server did not come up");
    server
}

#[test]
fn the_server_keeps_no_descriptor_it_inherited() {
    let dir = tempfile::tempdir().unwrap();
    let lease = dir.path().join("w.lease");
    let _window = fs_wiki_server::lease::hold(&lease).unwrap();
    let server = start(dir.path(), &lease);
    // The number 9 may be taken again by something the server opened itself; the inherited one points to /dev/null.
    let inherited: Vec<_> = std::fs::read_dir(format!("/proc/{}/fd", server.0.id()))
        .unwrap()
        .filter_map(|e| {
            let e = e.ok()?;
            let number: u32 = e.file_name().to_str()?.parse().ok()?;
            let target = std::fs::read_link(e.path()).ok()?;
            (number > 2 && target == Path::new("/dev/null")).then_some(number)
        })
        .collect();
    assert!(
        inherited.is_empty(),
        "the server still holds inherited descriptors {inherited:?}"
    );
}

#[test]
fn the_server_ends_when_the_folder_of_its_lease_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("run");
    std::fs::create_dir(&folder).unwrap();
    let lease = folder.join("w.lease");
    let window = fs_wiki_server::lease::hold(&lease).unwrap();
    let mut server = start(&folder, &lease);
    assert!(server.0.try_wait().unwrap().is_none(), "held: it goes on");
    std::fs::remove_dir_all(&folder).unwrap();
    drop(window);
    let began = Instant::now();
    let mut ended = None;
    while ended.is_none() && began.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(50));
        ended = server.0.try_wait().unwrap();
    }
    assert!(
        ended.is_some(),
        "the server outlived the folder of its lease"
    );
}
