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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::{read_envelope, read_object_id, Problem};
use fs_wiki_core::{Registry, Subscription, TextChange, TextFeed, TextListener};
use tokio::sync::oneshot;

use crate::feed_job::{deliver, settle, start};
use crate::feed_state::FeedState;
use crate::remote::Remote;
use crate::RpcTexts;

thread_local! {
    /// Set while a listener of the registry is being called: this thread is the connection's.
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// Run `f` as a callback of the connection's thread.
pub(crate) fn in_callback<T>(f: impl FnOnce() -> T) -> T {
    let before = IN_CALLBACK.with(|c| c.replace(true));
    let out = f();
    IN_CALLBACK.with(|c| c.set(before));
    out
}

/// End the feed of `generation`, if it is still the running one.
pub(crate) fn end(registry: &Registry, state: &FeedState, generation: u64, reason: &str) {
    if state.release(generation) {
        in_callback(|| registry.end(reason));
    }
}

/// The listener the server calls. When it is dropped the connection is over (or the server let go).
pub(crate) struct Feed {
    pub(crate) registry: Arc<Registry>,
    pub(crate) state: Arc<FeedState>,
    pub(crate) generation: u64,
    /// Set once the server has accepted the subscription. Before that the attempt is settled by the
    /// caller (`settle`), whatever the listener object does.
    pub(crate) established: Arc<AtomicBool>,
    /// Dropped with this value: lets the task that keeps the subscription alive end.
    pub(crate) _alive: oneshot::Sender<()>,
}

impl Drop for Feed {
    fn drop(&mut self) {
        if self.established.load(Ordering::SeqCst) {
            let reason = "the feed of the wiki server ended";
            end(&self.registry, &self.state, self.generation, reason);
        }
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

impl TextFeed for RpcTexts {
    /// Tell `listener` about every change until the returned subscription is dropped. The first
    /// call blocks for one round trip to the server (call it where `connect` may be called); from
    /// inside a listener callback it does not wait but queues the request. If the subscription
    /// fails, the listeners are told through `TextListener::ended`: "subscription failed" for one
    /// attempt (a later `subscribe` may try again), or that the connection is gone (then no
    /// `subscribe` can work, and `ended` is delivered from a thread of its own, one listener at a
    /// time, never from inside `subscribe`).
    fn subscribe(&self, listener: Arc<dyn TextListener>) -> Subscription {
        let subscription = self.registry.add(listener.clone());
        if self.state.is_dead() {
            if self.state.first_notice(&listener) && self.state.queue_notice(listener) {
                let state = self.state.clone();
                std::thread::spawn(move || deliver(&state));
            }
            return subscription;
        }
        let Some(generation) = self.state.claim() else {
            return subscription;
        };
        let (registry, state) = (self.registry.clone(), self.state.clone());
        let job = {
            let (registry, state) = (registry.clone(), state.clone());
            move |remote: Remote| start(remote, registry, state, generation)
        };
        if IN_CALLBACK.with(Cell::get) {
            // On the connection's thread waiting would wait for ourselves: another thread waits.
            let worker = self.worker().clone();
            std::thread::spawn(move || {
                let outcome = futures::executor::block_on(worker.call(job));
                settle(outcome, (&registry, &state, generation));
            });
        } else {
            let outcome = futures::executor::block_on(self.worker().call(job));
            settle(outcome, (&registry, &state, generation));
        }
        subscription
    }
}
