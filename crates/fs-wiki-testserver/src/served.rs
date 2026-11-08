//! `fs-wiki-testserver`: the real `fs-wiki-server` for tests, on a real Unix socket over a real store,
//! on a thread of its own. Nothing here stands in for the server; `serving` only lets a test put a
//! provider of its own in front of the real feed (for what the real one cannot be made to do).
//!
//! Pattern: RAII guard — dropping a [`Served`] stops the server and joins its thread, which also
//! lets go of the store file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_interfaces::rpc::{bind_socket, serve};
use fs_objects::{Store, TursoStore};
use fs_wiki_core::{TextFeed, Texts, WikiRepo};
use fs_wiki_server::TextsServer;
use tokio::sync::oneshot;

const ACTOR: &str = "https://local.invalid/users/owner";

/// A server running until this is dropped.
pub struct Served {
    /// The socket it listens on.
    pub socket: PathBuf,
    /// The provider behind the server, for what a test sets up or checks on the side.
    pub repo: Arc<WikiRepo>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

impl Served {
    /// Over a fresh in-memory store.
    ///
    /// # Panics
    /// The store or the socket cannot be made.
    pub async fn in_memory() -> Self {
        Self::over(Arc::new(TursoStore::open_in_memory().await.unwrap()))
    }

    /// Over a store file (it survives this value: serve it again to "restart").
    ///
    /// # Panics
    /// The store or the socket cannot be made.
    pub async fn with_file(file: &Path) -> Self {
        Self::over(Arc::new(TursoStore::open(file).await.unwrap()))
    }

    /// Over this store.
    ///
    /// # Panics
    /// The socket cannot be made.
    #[must_use]
    pub fn over(store: Arc<dyn Store>) -> Self {
        // The port is the real one, pointed at a socket in it, where nobody listens: the tests never reach a hub,
        // so `purge` answers "no hub" and the report offers the same ids as the real server.
        // A private folder of our own (a fixed name in the shared temp folder could be bound by another user).
        let none = tempfile::tempdir().unwrap().keep().join("no-hub.sock");
        let port = fs_wiki_server::HubPurge::new(store.clone(), Some(none));
        let repo = Arc::new(WikiRepo::new(store, ACTOR).with_log_purge(Arc::new(port)));
        Self::serving(repo.clone(), repo)
    }

    /// A server whose `Texts` is `texts` and whose feed is that of `repo`.
    ///
    /// # Panics
    /// The socket cannot be made.
    #[must_use]
    pub fn serving(texts: Arc<dyn Texts>, repo: Arc<WikiRepo>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("wiki.sock");
        let (stop, stopped) = oneshot::channel::<()>();
        let (ready, listening) = std::sync::mpsc::channel();
        let (path, feed) = (socket.clone(), repo.clone() as Arc<dyn TextFeed>);
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async move {
                let bound = bind_socket(&path).await.unwrap();
                let root = TextsServer::new(texts, feed);
                ready.send(()).unwrap();
                let shutdown = async {
                    let _ = stopped.await;
                };
                serve(bound, root.into_client().client, shutdown)
                    .await
                    .unwrap();
            });
        });
        listening.recv().unwrap();
        Self {
            socket,
            repo,
            stop: Some(stop),
            thread: Some(thread),
            _dir: dir,
        }
    }

    /// Stop the server; clients stay and find it gone.
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.stop();
    }
}
