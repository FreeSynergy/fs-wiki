//! The real `fs-wiki-server` for the tests: on a real Unix socket, over the real store, on a thread
//! of its own. Nothing here stands in for the server.
//!
//! Pattern: RAII guard — dropping it stops the server and joins its thread.

use std::path::PathBuf;
use std::sync::Arc;

use fs_interfaces::rpc::{bind_socket, serve};
use fs_store::Store;
use fs_wiki_core::WikiRepo;
use fs_wiki_server::TextsServer;
use tokio::sync::oneshot;

const ACTOR: &str = "https://example.org/actors/t";

/// A server running until this is dropped.
pub struct Served {
    pub socket: PathBuf,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

impl Served {
    /// Serve a wiki over `store`.
    pub fn over(store: Arc<dyn Store>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("wiki.sock");
        let (stop, stopped) = oneshot::channel::<()>();
        let (ready, listening) = std::sync::mpsc::channel();
        let path = socket.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async move {
                let bound = bind_socket(&path).await.unwrap();
                let repo = Arc::new(WikiRepo::new(store, ACTOR));
                let root = TextsServer::new(repo.clone(), repo).into_client();
                ready.send(()).unwrap();
                let shutdown = async {
                    let _ = stopped.await;
                };
                serve(bound, root.client, shutdown).await.unwrap();
            });
        });
        listening.recv().unwrap();
        Self {
            socket,
            stop: Some(stop),
            thread: Some(thread),
            _dir: dir,
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
