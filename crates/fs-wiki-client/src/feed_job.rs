//! The job that makes the remote subscription, and what comes of it.
//!
//! Pattern: Observer (making the remote subscription and delivering the end of a feed to the
//! listeners, one at a time from a queue) + Command (`start` is the job for the connection's thread).
//! Free functions with effects: they mark the connection dead, end the feed and start a delivery
//! thread; the rules are in `feed.rs`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::change_listener;
use fs_interfaces::rpc::client::Fault;
use fs_wiki_core::Registry;
use tokio::sync::oneshot;

use crate::feed::{end, in_callback, Feed};
use crate::feed_state::{FeedState, DEAD, NOTICE_PAUSE};
use crate::remote::Remote;

/// Why a remote subscription could not be made.
#[derive(Debug)]
pub(crate) enum Failed {
    /// The connection is gone: no feed can be made on it any more.
    Dead(String),
    /// This one attempt failed (the server refused, or answered too late); a later one may work.
    Once(String),
}

/// Subscribe on the connection's thread and keep the subscription until the listener is dropped.
async fn subscribe_remote(
    remote: &Remote,
    registry: &Arc<Registry>,
    state: &Arc<FeedState>,
    generation: u64,
) -> Result<(), capnp::Error> {
    let (alive, over) = oneshot::channel::<()>();
    let established = Arc::new(AtomicBool::new(false));
    let listener: change_listener::Client = new_client(Feed {
        registry: registry.clone(),
        state: state.clone(),
        generation,
        established: established.clone(),
        _alive: alive,
    });
    let mut request = remote.texts.subscribe_request();
    request.get().set_listener(listener);
    let answer = request.send().promise.await?;
    let subscription = answer.get()?.get_subscription()?;
    established.store(true, Ordering::SeqCst);
    // The capability and the response that carries it both keep the subscription alive.
    tokio::task::spawn_local(async move {
        let _keep = (answer, subscription);
        let _ = over.await;
    });
    Ok(())
}

/// The whole job, on the connection's thread.
pub(crate) async fn start(
    remote: Remote,
    registry: Arc<Registry>,
    state: Arc<FeedState>,
    generation: u64,
) -> Result<(), Failed> {
    subscribe_remote(&remote, &registry, &state, generation)
        .await
        .map_err(|e| match e.kind {
            capnp::ErrorKind::Disconnected => Failed::Dead(e.to_string()),
            _ => Failed::Once(e.to_string()),
        })
}

/// What came of the job: nothing to do, or the feed of this generation ends. Only a connection that
/// is really gone is marked dead; a deadline, an abort or a refusal fails this attempt alone.
pub(crate) fn settle(
    outcome: Result<Result<(), Failed>, Fault>,
    (registry, state, generation): (&Registry, &FeedState, u64),
) {
    let (dead, why) = match outcome {
        Ok(Ok(())) => return,
        Ok(Err(Failed::Dead(why))) => (true, why),
        Ok(Err(Failed::Once(why))) => (false, why),
        Err(fault @ Fault::Gone { .. }) => (true, fault.to_string()),
        Err(fault) => (false, fault.to_string()),
    };
    if dead {
        state.bury();
        end(registry, state, generation, &format!("{DEAD} ({why})"));
    } else {
        end(
            registry,
            state,
            generation,
            &format!("subscription failed: {why}"),
        );
    }
}

/// Tell the queued listeners that the feed is dead, one after the other, with a pause between.
pub(crate) fn deliver(state: &FeedState) {
    while let Some(listener) = state.next_notice() {
        in_callback(|| listener.ended(DEAD));
        std::thread::sleep(NOTICE_PAUSE);
    }
}
