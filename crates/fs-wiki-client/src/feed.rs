//! `TextFeed` over the socket.
//!
//! Pattern: Observer + Adapter — the connection holds one remote subscription (`subscribe` of the
//! contract), made when the first listener comes, and tells a local `Registry` of the core, so any
//! number of listeners share it and each ends with its own `Subscription` (RAII). The `ChangeListener`
//! below turns a wire `Change` into a `TextChange`. When the feed ends without the listener asking
//! (the server said `ended`, or the connection is gone), the registry tells every listener through
//! `TextListener::ended`; subscribing again then starts a new remote subscription. Dropping a
//! `Subscription` leaves the remote one running until the connection ends.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::{read_envelope, read_object_id, Problem};
use fs_wiki_core::{Registry, Subscription, TextChange, TextFeed, TextListener};
use tokio::sync::oneshot;

use crate::remote::Remote;
use crate::RpcTexts;

/// The listener the server calls. When it is dropped the connection is over (or the server let go).
struct Feed {
    registry: Arc<Registry>,
    started: Arc<AtomicBool>,
    /// Dropped with this value: lets the task that keeps the subscription alive end.
    _alive: oneshot::Sender<()>,
}

impl Feed {
    fn end(&self, reason: &str) {
        end(&self.registry, &self.started, reason);
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.end("the connection to the wiki server ended");
    }
}

fn end(registry: &Registry, started: &AtomicBool, reason: &str) {
    started.store(false, Ordering::SeqCst);
    registry.end(reason);
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
        self.registry.notify(&told);
        Ok(())
    }

    async fn ended(
        self: Rc<Self>,
        params: change_listener::EndedParams,
        _: change_listener::EndedResults,
    ) -> Result<(), capnp::Error> {
        let problem = Problem::read(params.get()?.get_problem()?)?;
        self.end(&format!(
            "the wiki server ended the feed: {}",
            problem.detail
        ));
        Ok(())
    }
}

/// Subscribe on the connection's thread and keep the subscription until the listener is dropped.
async fn subscribe_remote(
    remote: Remote,
    registry: Arc<Registry>,
    started: Arc<AtomicBool>,
) -> Result<(), String> {
    let (alive, over) = oneshot::channel::<()>();
    let listener: change_listener::Client = new_client(Feed {
        registry,
        started,
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

impl TextFeed for RpcTexts {
    /// Tell `listener` about every change until the returned subscription is dropped. The first
    /// call blocks for one round trip to the server (call it where `connect` may be called). If the
    /// server cannot be reached the listener is told at once through `TextListener::ended`.
    fn subscribe(&self, listener: Arc<dyn TextListener>) -> Subscription {
        let subscription = self.registry.add(listener);
        if !self.started.swap(true, Ordering::SeqCst) {
            let (registry, started) = (self.registry.clone(), self.started.clone());
            let work = {
                let (registry, started) = (registry.clone(), started.clone());
                move |remote: Remote| subscribe_remote(remote, registry, started)
            };
            let outcome = futures::executor::block_on(self.worker().call(work));
            let failed = match outcome {
                Ok(Ok(())) => return subscription,
                Ok(Err(text)) => text,
                Err(fault) => fault.to_string(),
            };
            end(&registry, &started, &format!("cannot subscribe: {failed}"));
        }
        subscription
    }
}
