//! `TextFeed` over the socket.
//!
//! Pattern: Observer + Adapter — the connection holds one remote subscription (`subscribe` of the
//! contract), made when the first listener comes, and tells a local `Registry` of the core, so any
//! number of listeners share it and each ends with its own `Subscription` (RAII). The `ChangeListener`
//! below turns a wire `Change` into a `TextChange`. When the feed ends without the listener asking
//! (the server said `ended`, or the connection is gone), the registry tells every listener through
//! `TextListener::ended`; subscribing again then starts a new remote subscription. Every remote
//! subscription has a generation number (`FeedState`): the end of an old one (its listener object is
//! dropped some time after `ended`) never touches a newer one. Dropping a `Subscription` leaves the
//! remote one running until the connection ends. Subscribing from inside a listener callback (which
//! runs on the connection's thread) cannot wait for the server there, so it only queues the request.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::{read_envelope, read_object_id, Problem};
use fs_wiki_core::{Registry, Subscription, TextChange, TextFeed, TextListener};
use futures::FutureExt;
use tokio::sync::oneshot;

use crate::remote::Remote;
use crate::RpcTexts;

/// Which remote subscription of a connection is the running one (0 = none).
#[derive(Default)]
pub(crate) struct FeedState {
    next: AtomicU64,
    current: AtomicU64,
}

impl FeedState {
    /// Take the right to start a remote subscription, if none is running.
    fn claim(&self) -> Option<u64> {
        let generation = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        self.current
            .compare_exchange(0, generation, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| generation)
    }

    /// Give up `generation`; false if it is not the running one any more.
    fn release(&self, generation: u64) -> bool {
        self.current
            .compare_exchange(generation, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

thread_local! {
    /// Set while a listener of the registry is being called: this thread is the connection's.
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// Run `f` as a callback of the connection's thread.
fn in_callback<T>(f: impl FnOnce() -> T) -> T {
    let before = IN_CALLBACK.with(|c| c.replace(true));
    let out = f();
    IN_CALLBACK.with(|c| c.set(before));
    out
}

/// End the feed of `generation`, if it is still the running one.
fn end(registry: &Registry, state: &FeedState, generation: u64, reason: &str) {
    if state.release(generation) {
        in_callback(|| registry.end(reason));
    }
}

/// The listener the server calls. When it is dropped the connection is over (or the server let go).
struct Feed {
    registry: Arc<Registry>,
    state: Arc<FeedState>,
    generation: u64,
    /// Dropped with this value: lets the task that keeps the subscription alive end.
    _alive: oneshot::Sender<()>,
}

impl Drop for Feed {
    fn drop(&mut self) {
        let reason = "the connection to the wiki server ended";
        end(&self.registry, &self.state, self.generation, reason);
    }
}

impl change_listener::Server for Feed {
    async fn change(
        self: Rc<Self>,
        params: change_listener::ChangeParams,
    ) -> Result<(), capnp::Error> {
        let told = match params.get()?.get_change()?.which()? {
            change::Which::Created(e) => TextChange::Created(read_envelope(e?)?.id),
            change::Which::Updated(e) => TextChange::Updated(read_envelope(e?)?.id),
            change::Which::Deleted(id) => TextChange::Deleted(read_object_id(id?)?),
        };
        in_callback(|| self.registry.notify(&told));
        Ok(())
    }

    async fn ended(
        self: Rc<Self>,
        params: change_listener::EndedParams,
        _: change_listener::EndedResults,
    ) -> Result<(), capnp::Error> {
        let problem = Problem::read(params.get()?.get_problem()?)?;
        let reason = format!("the wiki server ended the feed: {}", problem.detail);
        end(&self.registry, &self.state, self.generation, &reason);
        Ok(())
    }
}

/// Subscribe on the connection's thread and keep the subscription until the listener is dropped.
async fn subscribe_remote(
    remote: &Remote,
    registry: &Arc<Registry>,
    state: &Arc<FeedState>,
    generation: u64,
) -> Result<(), String> {
    let (alive, over) = oneshot::channel::<()>();
    let listener: change_listener::Client = new_client(Feed {
        registry: registry.clone(),
        state: state.clone(),
        generation,
        _alive: alive,
    });
    let mut request = remote.texts.subscribe_request();
    request.get().set_listener(listener);
    let answer = request.send().promise.await.map_err(|e| e.to_string())?;
    let subscription = answer
        .get()
        .and_then(|r| r.get_subscription())
        .map_err(|e| e.to_string())?;
    // The capability and the response that carries it both keep the subscription alive.
    tokio::task::spawn_local(async move {
        let _keep = (answer, subscription);
        let _ = over.await;
    });
    Ok(())
}

/// The whole job: subscribe, and end the feed of this generation if that fails.
async fn start(remote: Remote, registry: Arc<Registry>, state: Arc<FeedState>, generation: u64) {
    if let Err(why) = subscribe_remote(&remote, &registry, &state, generation).await {
        end(
            &registry,
            &state,
            generation,
            &format!("cannot subscribe: {why}"),
        );
    }
}

impl TextFeed for RpcTexts {
    /// Tell `listener` about every change until the returned subscription is dropped. The first
    /// call blocks for one round trip to the server (call it where `connect` may be called); from
    /// inside a listener callback it does not wait but queues the request. If the server cannot be
    /// reached the listener is told through `TextListener::ended`.
    fn subscribe(&self, listener: Arc<dyn TextListener>) -> Subscription {
        let subscription = self.registry.add(listener);
        let Some(generation) = self.state.claim() else {
            return subscription;
        };
        let (registry, state) = (self.registry.clone(), self.state.clone());
        let job = {
            let (registry, state) = (registry.clone(), state.clone());
            move |remote: Remote| start(remote, registry, state, generation)
        };
        let call = self.worker().call(job);
        if IN_CALLBACK.with(Cell::get) {
            // On the connection's thread waiting would wait for ourselves: polling once queues the job.
            let _ = Box::pin(call).now_or_never();
        } else if let Err(fault) = futures::executor::block_on(call) {
            end(
                &registry,
                &state,
                generation,
                &format!("cannot subscribe: {fault}"),
            );
        }
        subscription
    }
}
