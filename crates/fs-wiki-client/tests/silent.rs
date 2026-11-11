//! The version handshake and the deadline of the calls: a server of this version is used as it is, and one that
//! passes the handshake and then says nothing does not keep the window waiting for ever.
//!
//! Pattern: none (tests) — a real `TextsServer` served through `run_until_signal_as` (so `Control.version` answers), over a
//! provider that never answers `get`. Own file: the servers are never stopped, they end with the process.

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use fs_interfaces::rpc::{run_until_signal_as, Expected, ServerIdentity};
use fs_interfaces::Capabilities;
use fs_objects::{ObjectId, TursoStore};
use fs_wiki_client::{RpcTexts, Timeouts};
use fs_wiki_core::{Paging, Place, Result, TextEntry, TextPage, TextQuery, Texts, WikiRepo};
use fs_wiki_server::TextsServer;

const LIMIT: Duration = Duration::from_millis(400);
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The real wiki, except that `get` never answers.
struct Hung(Arc<WikiRepo>);

#[async_trait]
impl Texts for Hung {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    async fn get(&self, _: ObjectId) -> Result<Option<TextEntry>> {
        std::future::pending().await
    }
    async fn page(&self, q: &TextQuery, after: Option<&Place>, size: usize) -> Result<TextPage> {
        self.0.page(q, after, size).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.0.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.0.list(prefix, page).await
    }
}

fn serve(socket: &Path) {
    let socket = socket.to_path_buf();
    let (up, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let identity = ServerIdentity::new("fs-wiki", VERSION);
        let _ = run_until_signal_as(&socket, identity, || async move {
            let store = TursoStore::open_in_memory()
                .await
                .map_err(io::Error::other)?;
            let repo = Arc::new(WikiRepo::new(
                Arc::new(store),
                "https://local.invalid/users/owner",
            ));
            up.send(()).ok();
            let texts = Arc::new(Hung(repo.clone()));
            Ok(TextsServer::new(texts, repo).into_client().client)
        });
    });
    started.recv().unwrap();
}

fn refuse() -> io::Result<()> {
    Err(io::Error::other("must not be started"))
}

fn short() -> Timeouts {
    Timeouts {
        call: LIMIT,
        ..Timeouts::default()
    }
}

#[test]
fn a_server_of_this_version_is_used_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("wiki.sock");
    serve(&socket);
    let texts = RpcTexts::connect_matching(&socket, &Expected::new(VERSION), &refuse, short());
    assert!(texts.is_ok(), "{:?}", texts.err());
}

#[test]
fn no_server_and_no_start_is_a_handshake_error_with_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("wiki.sock");
    let error = RpcTexts::connect_matching(&socket, &Expected::new(VERSION), &refuse, short())
        .err()
        .expect("no server, no start");
    assert!(error.to_string().contains("must not be started"), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_that_the_server_never_answers_ends_with_the_limit_and_the_connection_stays_usable()
{
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("wiki.sock");
    serve(&socket);
    // The connection is made before the window opens, not from inside a task (it blocks the thread).
    let texts = tokio::task::spawn_blocking(move || {
        RpcTexts::connect_matching(&socket, &Expected::new(VERSION), &refuse, short())
            .map_err(|e| e.to_string())
    })
    .await
    .unwrap()
    .unwrap();
    for _ in 0..2 {
        let began = Instant::now();
        let got = texts.get(ObjectId::new()).await;
        let took = began.elapsed();
        let text = format!("{got:?}");
        assert!(got.is_err(), "{text}");
        assert!(text.contains("did not answer"), "{text}");
        assert!(took >= LIMIT, "too early: {took:?}");
        assert!(took < LIMIT * 5, "the limit did not hold: {took:?}");
    }
}
