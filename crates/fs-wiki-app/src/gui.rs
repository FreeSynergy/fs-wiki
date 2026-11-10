//! The window of the wiki program: connect to the server (starting it when none runs), then the Admin window, the
//! headless check or the reader.
//!
//! Pattern: Facade — one entry point over the arguments, the server start and the two modes. A server the window starts
//! ends itself when no window holds the lease any more (`fs-wiki-server` `lease.rs`); a socket given with `--socket` is
//! never answered by starting anything.

use std::path::Path;

use fs_wiki_client::RpcTexts;

use crate::args::{HELP, parse};
use crate::server::socket_path;
use crate::words::{self, say};

/// Run the window program with the words after the program name.
///
/// # Errors
/// A message for the person.
pub fn run(words: &[String]) -> Result<(), String> {
    let args = parse(words)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let socket = socket_path(args.socket.as_deref())?;
    let texts = connect_or_start(&socket, args.socket.is_none())?;
    if args.reader {
        crate::mode_reader::run(&args, texts)
    } else {
        crate::mode_admin::run(&args, texts)
    }
}

fn connect_error(socket: &Path, e: &std::io::Error) -> String {
    say(
        words::SERVER_UNREACHABLE,
        &[("path", &socket.display()), ("error", e)],
    )
}

/// Connect; if none answers on the user's own socket, start `<this program> serve` (detached, so other windows use it
/// too) and wait for it.
fn connect_or_start(socket: &Path, may_start: bool) -> Result<RpcTexts, String> {
    // This window is a user of the server for as long as it lives: the shared lock is let go by the system when the
    // process ends. A server this window starts ends itself when no window holds the lease.
    let lease = socket.with_extension("lease");
    if may_start {
        hold_lease(&lease);
    }
    let first = RpcTexts::connect(socket);
    if first.is_ok() || !may_start {
        return first.map_err(|e| connect_error(socket, &e));
    }
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    // What the server says when it cannot start goes to a file next to the socket, so the window can tell the person.
    let log = socket.with_extension("log");
    let log_file = std::fs::File::create(&log).ok();
    let mut server = std::process::Command::new(me);
    server
        .arg("serve")
        .arg("--lease")
        .arg(&lease)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(log_file.map_or_else(std::process::Stdio::null, std::process::Stdio::from));
    std::os::unix::process::CommandExt::process_group(&mut server, 0);
    let mut child = server
        .spawn()
        .map_err(|e| format!("could not start the wiki server: {e}"))?;
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Ok(texts) = RpcTexts::connect(socket) {
            let _ = std::fs::remove_file(&log);
            return Ok(texts);
        }
        if let Ok(Some(status)) = child.try_wait() {
            let said = std::fs::read_to_string(&log).unwrap_or_default();
            let _ = std::fs::remove_file(&log);
            return Err(format!(
                "the wiki server stopped at once ({status}): {}",
                said.trim()
            ));
        }
    }
    // Still waiting: the child lives on (it may just be slow); reap it if it ends later.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    first.map_err(|e| connect_error(socket, &e))
}

/// Hold the shared lock on `path` until this process ends.
fn hold_lease(path: &Path) {
    static HELD: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        && file.lock_shared().is_ok()
    {
        let _ = HELD.set(file);
    }
}
