//! `fs-wiki serve`: the options, the store and the serving, as a function so one binary can hold server and window.
//!
//! Pattern: Facade — reads the options, opens the store, serves until a signal. `[--lease FILE]` ends the server when no
//! window holds the lease any more (`lease.rs`).

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{HubPurge, TextsServer};
use fs_interfaces::rpc::{
    prepare_data_dir, run_until_signal_as, stop_self, ServerIdentity, ServerOptions,
};
use fs_objects::{Store, TursoStore};
use fs_wiki_core::WikiRepo;

/// The help text.
pub const HELP: &str = "fs-wiki serve [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN] [--facet NAME] [--hub-socket PATH] [--lease FILE]\nDefaults: $XDG_RUNTIME_DIR/freesynergy/wiki.sock, $XDG_DATA_HOME/freesynergy/wiki, the local owner; the hub at its default socket (only `purge` and `purgeRevision` need it)\n--lease FILE: end when no window holds this lock any more (started by a window, not by hand)";

/// Why `serve` ended without serving.
#[derive(Debug, PartialEq, Eq)]
pub enum ServeError {
    /// The words were wrong; the caller exits with 2.
    Usage(String),
    /// The server could not start or stopped with an error; exit with 1.
    Failed(String),
}

/// Serve `texts.capnp` until a signal arrives. `args` are the words after `serve`.
///
/// # Errors
/// [`ServeError::Usage`] for unknown or incomplete words, [`ServeError::Failed`] for everything else.
pub fn serve(args: impl IntoIterator<Item = OsString>) -> Result<(), ServeError> {
    let (lease, args) =
        crate::lease::take_lease(args.into_iter().collect()).map_err(ServeError::Usage)?;
    let (hub_socket, args) = hub_socket_of(args);
    let Some(ServerOptions {
        socket,
        data,
        actor,
        instance,
        // The facet is read by the shared parser; this server serves one wiki whatever the link says.
        facet: _,
    }) = ServerOptions::parse("wiki", args).map_err(ServeError::Usage)?
    else {
        println!("{HELP}");
        return Ok(());
    };
    if let Some(lease) = lease {
        // A window started this server and goes away before it: nothing the window passed on stays open here.
        crate::detach::close_inherited();
        // The window that started this server holds the lease; when none does, end as a signal would (Windows has no
        // signal to send, so the server is asked from inside).
        crate::lease::watch(lease, stop_self);
    }
    // Name and version go to `Control.version`: a window of another version replaces this server (fs-interfaces handshake).
    let identity = ServerIdentity::new("fs-wiki", env!("CARGO_PKG_VERSION"));
    run_until_signal_as(&socket, identity, || async move {
        let data = prepare_data_dir(&data)?;
        let store = TursoStore::open(data.join("objects.db"))
            .await
            .map_err(std::io::Error::other)?;
        let store: Arc<dyn Store> = Arc::new(store);
        let mut repo = WikiRepo::new(store.clone(), actor)
            .with_log_purge(Arc::new(HubPurge::new(store, hub_socket)));
        if let Some(instance) = instance {
            repo = repo.with_instance(instance);
        }
        // Texts stored before 0.3.0 get the index fields the pages need; what cannot be repaired is
        // named, and a page says so as well.
        let report = repo.upgrade_index().await.map_err(std::io::Error::other)?;
        if report.rewritten > 0 {
            eprintln!("fs-wiki: index completed for {} texts", report.rewritten);
        }
        for failed in &report.failed {
            eprintln!("fs-wiki: text not repaired: {failed}");
        }
        // A file clean-up that an earlier purge could not finish is done now, if the hub is not needed.
        if let Ok(false) = repo.retry_file_cleanup().await {
            eprintln!("fs-wiki: a file clean-up after a purge is still pending");
        }
        let repo = Arc::new(repo);
        Ok(TextsServer::new(repo.clone(), repo).into_client().client)
    })
    .map_err(|e| ServeError::Failed(e.to_string()))
}

/// `--hub-socket PATH` taken out of the arguments; the rest goes to the shared option parser.
fn hub_socket_of(args: Vec<OsString>) -> (Option<PathBuf>, Vec<OsString>) {
    let (mut socket, mut rest) = (None, vec![]);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--hub-socket" {
            socket = args.next().map(PathBuf::from);
        } else {
            rest.push(arg);
        }
    }
    (socket, rest)
}
