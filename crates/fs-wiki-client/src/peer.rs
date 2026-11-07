//! The connection: what a frontend holds.
//!
//! Pattern: Adapter — `Texts` and `TextFeed` of the core in front of the Cap'n Proto client of
//! `texts.capnp`. The thread of the connection is the shared `Worker` of `fs-interfaces`, with the
//! contract in `remote.rs`; the calls in `calls/`; the traits in `texts_impl.rs` and `feed.rs`.
//! Interface first (ADR 0043): `connect` and `connect_with` were written down before the calls.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_interfaces::rpc::client::{Fault, Timeouts, Worker};
use fs_interfaces::Capabilities;
use fs_wiki_core::{Error, Registry, Result};

use crate::feed::FeedState;
use crate::remote::Remote;

/// The name of the server in every error text (`wiki server: ...`).
const SERVER: &str = "wiki";

/// The wiki program as seen over its socket. Implements `Texts` and `TextFeed`.
///
/// Calls from any thread are fine; they queue for the one thread that owns the connection and
/// overlap there. Dropping it closes the connection.
pub struct RpcTexts {
    worker: Arc<Worker<Remote>>,
    capabilities: Capabilities,
    /// The listeners of `TextFeed::subscribe`: the one remote subscription tells this registry.
    pub(crate) registry: Arc<Registry>,
    /// Which remote subscription is the running one (see `feed.rs`).
    pub(crate) state: Arc<FeedState>,
}

impl RpcTexts {
    /// Connect to the `fs-wiki-server` that listens on `socket` and ask what it can do.
    ///
    /// Blocks the calling thread until the server has answered; call it before the window opens, not
    /// from inside an async task of a single-threaded runtime.
    ///
    /// # Errors
    /// Nothing listens there, the server runs as another user, or it does not answer the contract.
    pub fn connect(socket: &Path) -> io::Result<Self> {
        Self::connect_with(socket, Timeouts::default())
    }

    /// [`connect`](Self::connect) with other deadlines than [`CONNECT_TIMEOUT`](crate::CONNECT_TIMEOUT)
    /// and [`CALL_TIMEOUT`](crate::CALL_TIMEOUT). A server that stays silent ends the wait with an
    /// error that names the deadline.
    ///
    /// # Errors
    /// As for [`connect`](Self::connect), and the server did not answer in time.
    pub fn connect_with(socket: &Path, timeouts: Timeouts) -> io::Result<Self> {
        let (worker, capabilities) = Worker::start(SERVER, socket, timeouts)?;
        Ok(Self {
            worker: Arc::new(worker),
            capabilities,
            registry: Arc::new(Registry::default()),
            state: Arc::default(),
        })
    }

    /// Where `fs-wiki-server` listens when it is given no `--socket`:
    /// `$XDG_RUNTIME_DIR/freesynergy/wiki.sock`. `None` when the environment names no runtime folder.
    #[must_use]
    pub fn default_socket() -> Option<PathBuf> {
        fs_interfaces::rpc::default_socket(SERVER)
    }

    /// What the server reported when this connection was made. A snapshot: the server is not asked
    /// again.
    pub(crate) fn report(&self) -> Capabilities {
        self.capabilities.clone()
    }

    /// The worker, for the one place that needs it beyond `call` (the first subscription).
    pub(crate) fn worker(&self) -> &Arc<Worker<Remote>> {
        &self.worker
    }

    /// Run `work` on the connection's thread and wait for its answer. A call that got none (the
    /// connection is gone, the job ended, the deadline ran out) is a `Backend` error with the text of
    /// the `Fault`: the core has no variant for it.
    pub(crate) async fn call<T, F, Fut>(&self, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(Remote) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + 'static,
    {
        let fault = |f: Fault| Error::Store(fs_store::Error::Backend(f.to_string()));
        self.worker.call(work).await.map_err(fault)?
    }
}
