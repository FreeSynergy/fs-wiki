//! A server of another version that cannot be replaced: the person gets the words of `HandshakeError`, both versions in it.
//!
//! Pattern: none (tests). Own file: the handshake stops the old server (`Control.stop`), one flag per process.

use std::io;
use std::sync::Arc;

use fs_interfaces::rpc::{run_until_signal_as, Expected, ServerIdentity};
use fs_objects::TursoStore;
use fs_wiki_client::{RpcTexts, Timeouts};
use fs_wiki_core::WikiRepo;
use fs_wiki_server::TextsServer;

#[test]
fn an_old_server_that_cannot_be_replaced_is_reported_with_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("wiki.sock");
    let (path, (up, started)) = (socket.clone(), std::sync::mpsc::channel());
    std::thread::spawn(move || {
        let identity = ServerIdentity::new("fs-wiki", "0.0.1");
        let _ = run_until_signal_as(&path, identity, || async move {
            let store = TursoStore::open_in_memory()
                .await
                .map_err(io::Error::other)?;
            let repo = Arc::new(WikiRepo::new(
                Arc::new(store),
                "https://local.invalid/users/owner",
            ));
            up.send(()).ok();
            Ok(TextsServer::new(repo.clone(), repo).into_client().client)
        });
    });
    started.recv().unwrap();
    let cannot = || -> io::Result<()> { Err(io::Error::other("cannot start here")) };
    let error = RpcTexts::connect_matching(
        &socket,
        &Expected::new(env!("CARGO_PKG_VERSION")),
        &cannot,
        Timeouts::default(),
    )
    .err()
    .expect("the old server is not replaced");
    let text = error.to_string();
    assert!(text.contains("0.0.1"), "what runs: {text}");
    assert!(
        text.contains(env!("CARGO_PKG_VERSION")),
        "what is needed: {text}"
    );
}
