//! `fs-wiki-server`. Usage: `fs-wiki-server [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN]
//! [--hub-socket PATH]`. The hub is only needed to purge (default socket of the hub).
//!
//! Pattern: none (thin binary) — reads its options, opens the store, serves until a signal.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use fs_interfaces::rpc::{prepare_data_dir, run_until_signal, ServerOptions};
use fs_objects::{Store, TursoStore};
use fs_wiki_core::WikiRepo;
use fs_wiki_server::{HubPurge, TextsServer};

fn main() {
    let (hub_socket, args) = hub_socket_of(std::env::args_os().skip(1).collect());
    let ServerOptions {
        socket,
        data,
        actor,
        instance,
    } = options(args);
    let served = run_until_signal(&socket, || async move {
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
            eprintln!(
                "fs-wiki-server: index completed for {} texts",
                report.rewritten
            );
        }
        for failed in &report.failed {
            eprintln!("fs-wiki-server: text not repaired: {failed}");
        }
        let repo = Arc::new(repo);
        Ok(TextsServer::new(repo.clone(), repo).into_client().client)
    });
    if let Err(e) = served {
        eprintln!("fs-wiki-server: {e}");
        std::process::exit(1);
    }
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

fn options(args: Vec<OsString>) -> ServerOptions {
    match ServerOptions::parse("wiki", args) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!(
                "fs-wiki-server [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN] [--hub-socket PATH]\nDefaults: $XDG_RUNTIME_DIR/freesynergy/wiki.sock, $XDG_DATA_HOME/freesynergy/wiki, the local owner; the hub at its default socket (only `purge` and `purgeRevision` need it)"
            );
            std::process::exit(0);
        }
        Err(problem) => {
            eprintln!("fs-wiki-server: {problem}");
            std::process::exit(2);
        }
    }
}
