//! The window of the wiki program: connect to the server (starting it when none runs), then the wiki window (the
//! normal start), the Admin window, the headless check or the reader.
//!
//! Pattern: Facade — one entry point over the arguments, the version handshake and the two modes. A server the window
//! starts ends itself when no window holds the lease any more (`fs-wiki-server` `lease.rs`); a server of another version
//! is replaced (`fs-interfaces` `connect_matching`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fs_interfaces::rpc::{CommandStarter, Expected, FACET_FLAG, companion_path};
use fs_wiki_client::{RpcTexts, Timeouts};

use crate::args::{HELP, parse};
use crate::server::socket_path;
use crate::words::{self, say};

/// Run the window program with the words after the program name.
///
/// # Errors
/// A message for the person.
pub fn run(words: &[String]) -> Result<(), String> {
    // The typing test reads what is typed into the fields: the registry must be on BEFORE the first field is made
    // (fields made earlier are not noted), so before the window is built. A debug-only feature, off in a release.
    #[cfg(feature = "test-access")]
    fs_components_widgets::test_access::start_from_env();
    let args = parse(words)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let socket = socket_path(args.socket.as_deref())?;
    let texts = connect_or_start(&socket, args.socket.is_some(), args.facet.as_deref())?;
    match mode(&args) {
        Mode::Wiki => crate::mode_read::run(&args, texts),
        Mode::Reader => crate::mode_reader::run(&args, texts),
        Mode::Admin => crate::mode_admin::run(&args, texts),
    }
}

/// Which window the arguments ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The wiki window with its frontends (no option, or `--read`).
    Wiki,
    /// The documentation reader and editor of one folder (`--reader`).
    Reader,
    /// The Admin window (`--admin`) or its headless check (`--check`).
    Admin,
}

/// The window of `args`: the wiki window unless `--reader`, `--admin` or `--check` ask for another (`--read` wins, as it
/// always did).
pub fn mode(args: &crate::args::Args) -> Mode {
    if args.read {
        Mode::Wiki
    } else if args.reader {
        Mode::Reader
    } else if args.admin || args.check {
        Mode::Admin
    } else {
        Mode::Wiki
    }
}

/// Connect to the server that fits this version (the handshake of `fs-interfaces`): one of this version is used as it is,
/// none is started (`<this program> serve`, detached, so other windows use it too), one of another version is ended and
/// replaced.
fn connect_or_start(
    socket: &Path,
    own_socket: bool,
    facet: Option<&str>,
) -> Result<RpcTexts, String> {
    // This window is a user of the server for as long as it lives: the shared lock is let go by the system when the
    // process ends. A server this window starts ends itself when no window holds the lease any more.
    let lease = companion_path(socket, "lease");
    hold_lease(&lease);
    let starter = starter(socket, &lease, own_socket, facet)?;
    let expected = Expected::new(env!("CARGO_PKG_VERSION"));
    RpcTexts::connect_matching(socket, &expected, &starter, timeouts()).map_err(|e| {
        say(
            words::HANDSHAKE,
            &[("path", &socket.display()), ("error", &e)],
        )
    })
}

/// How the server is started when none fits: `fs-wiki serve --lease <lease>` (or the `fs-wiki-server` next to this
/// window-only program), on the socket this window talks to. What it says when it cannot start goes to its own stderr,
/// which a detached server has not: the person sees the words of the handshake instead.
fn starter(
    socket: &Path,
    lease: &Path,
    own_socket: bool,
    facet: Option<&str>,
) -> Result<CommandStarter, String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = me.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    // `serve` is not a word of the window-only program: its server is the binary next to it.
    let (program, mut words): (PathBuf, Vec<OsString>) = if name.starts_with("fs-wiki-app") {
        (me.with_file_name("fs-wiki-server"), Vec::new())
    } else {
        (me, vec!["serve".into()])
    };
    if own_socket {
        words.extend(["--socket".into(), socket.as_os_str().to_owned()]);
    }
    words.extend(["--lease".into(), lease.as_os_str().to_owned()]);
    if let Some(facet) = facet {
        words.extend([FACET_FLAG.into(), facet.into()]);
    }
    Ok(CommandStarter::new(program, words))
}

/// Hold the shared lock on `path` until this process ends.
fn hold_lease(path: &Path) {
    static HELD: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    // The folder of the socket may not exist yet (the first start of all): the person's own, private (as the server makes it).
    if let Some(dir) = path.parent() {
        make_private_dir(dir);
    }
    // Read and write, not append: Windows takes a lock only on a handle that can read or write data.
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
    {
        Ok(file) => match file.lock_shared() {
            Ok(()) => {
                let _ = HELD.set(file);
            }
            Err(e) => eprintln!(
                "fs-wiki: no lease ({e}): a server this window starts ends by itself after a while"
            ),
        },
        Err(e) => eprintln!("fs-wiki: no lease file {} ({e})", path.display()),
    }
}

/// Make `dir` (and what lacks above it) readable by the owner only; a failure is not reported here, the lease then says so.
fn make_private_dir(dir: &Path) {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    let _ = builder.create(dir);
}

/// The deadlines of the calls: the defaults, or `FS_WIKI_CALL_TIMEOUT` seconds for a call (listing the whole documentation
/// takes longer than the default on a server that has not got it cached).
fn timeouts() -> Timeouts {
    let mut t = Timeouts::default();
    if let Some(secs) = std::env::var("FS_WIKI_CALL_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        t.call = std::time::Duration::from_secs(secs);
    }
    t
}
