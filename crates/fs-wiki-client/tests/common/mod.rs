//! Shared set-up of the tests: the real server on a real Unix socket over the real store, on a
//! thread of its own, and the client connected to it. Nothing here stands in for either.
//!
//! Pattern: Facade over the set-up.
#![allow(dead_code)] // each test file uses a different subset

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_interfaces::rpc::{bind_socket, serve};
use fs_store::{Store, TursoStore};
use fs_wiki_client::RpcTexts;
use fs_wiki_core::{TextEntry, TextFeed, Texts, WikiRepo};
use fs_wiki_server::TextsServer;
use tokio::sync::oneshot;

const ME: &str = "https://local.invalid/users/owner";

/// A server running until this is dropped.
pub struct Served {
    pub socket: PathBuf,
    /// The provider behind the server, for what a test sets up or checks on the side.
    pub repo: Arc<WikiRepo>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

impl Served {
    /// Over a fresh in-memory store.
    pub async fn start() -> Self {
        let store = TursoStore::open_in_memory().await.unwrap();
        Self::over(Arc::new(store)).await
    }

    /// Over a store file (it survives this value: start again over the same file to "restart").
    pub async fn start_with_file(file: &Path) -> Self {
        let store = TursoStore::open(file).await.unwrap();
        Self::over(Arc::new(store)).await
    }

    async fn over(store: Arc<dyn Store>) -> Self {
        let repo = Arc::new(WikiRepo::new(store, ME));
        Self::serving(repo.clone(), repo).await
    }

    /// A server whose `Texts` is this provider and whose feed comes from `repo`: for a provider
    /// that does what the real one cannot show, such as never answering.
    pub async fn serving(texts: Arc<dyn Texts>, repo: Arc<WikiRepo>) -> Self {
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

    /// A client of this server.
    pub fn client(&self) -> RpcTexts {
        RpcTexts::connect(&self.socket).unwrap()
    }

    /// Stop the server; clients stay and find it gone.
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Create a text through the client.
pub async fn make(texts: &dyn Texts, path: &str, markdown: &str) -> TextEntry {
    texts.create(path, markdown).await.unwrap()
}

/// The paths of all texts below `prefix`, as the client lists them.
pub async fn paths(texts: &dyn Texts, prefix: &str) -> Vec<String> {
    let all = texts
        .list(prefix, fs_wiki_core::Paging::default())
        .await
        .unwrap();
    all.into_iter().map(|t| t.path).collect()
}
