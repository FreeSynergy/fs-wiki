//! The [`TextFeed`] contract: live changes of texts (`text.subscribe`).
//!
//! Pattern: Observer — a [`TextListener`] subscribes and is told about every change that was written;
//! dropping the [`Subscription`] ends it (RAII, as in `common.capnp`). Why: the writer must not know
//! who listens, and a listener must not be able to stop a write. [`Registry`] is the reusable subject;
//! `fs-tasks-core` holds the same shape (to move into a shared crate, see `DESIGN.md`).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use fs_store::ObjectId;

/// What happened to a text. A move is an `Updated` of the moved text and of every text whose links
/// followed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextChange {
    /// A text was created (also by an import to a new path).
    Created(ObjectId),
    /// A text was written again (edit, move, rewritten links, import to an existing path).
    Updated(ObjectId),
    /// A text was deleted.
    Deleted(ObjectId),
}

/// Receives changes. Called on the thread that made the change, after the write: keep it short and
/// hand work off.
pub trait TextListener: Send + Sync {
    /// A text changed.
    fn changed(&self, change: &TextChange);

    /// The feed ended without the listener dropping its [`Subscription`]: a provider behind a
    /// connection that was lost or that stopped telling, so changes may have been missed. To go on,
    /// subscribe again and read the current state, in that order. Called on the thread that
    /// delivers changes: it may call `subscribe` again, but a provider behind a connection cannot
    /// wait for the server there, so that call only queues the request and reports a failure
    /// through `ended` again. Reading the state must wait until the callback has returned. A provider
    /// in the same process never ends a feed, so the default does nothing.
    fn ended(&self, _reason: &str) {}
}

/// Live updates of texts.
pub trait TextFeed: Send + Sync {
    /// Tell `listener` about every change until the returned [`Subscription`] is dropped.
    fn subscribe(&self, listener: Arc<dyn TextListener>) -> Subscription;
}

type Entries = Vec<(u64, Arc<dyn TextListener>)>;

/// The subject of the Observer: holds the listeners and notifies them.
#[derive(Default)]
pub struct Registry {
    next: Mutex<u64>,
    listeners: Mutex<Entries>,
}

impl Registry {
    /// Add a listener; the subscription removes it again.
    pub fn add(self: &Arc<Self>, listener: Arc<dyn TextListener>) -> Subscription {
        let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
        *next += 1;
        let id = *next;
        self.entries().push((id, listener));
        Subscription {
            id,
            registry: Arc::clone(self),
        }
    }

    /// Tell every listener.
    pub fn notify(&self, change: &TextChange) {
        let listeners: Vec<_> = self.entries().iter().map(|(_, l)| l.clone()).collect();
        for l in listeners {
            l.changed(change);
        }
    }

    /// End the feed: tell every listener why, and forget them. Their subscriptions stay valid to drop.
    pub fn end(&self, reason: &str) {
        let listeners: Vec<_> = self.entries().drain(..).map(|(_, l)| l).collect();
        for l in listeners {
            l.ended(reason);
        }
    }

    fn entries(&self) -> MutexGuard<'_, Entries> {
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Keeps a subscription alive; dropping it unsubscribes.
#[must_use = "dropping the subscription ends it"]
pub struct Subscription {
    id: u64,
    registry: Arc<Registry>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let id = self.id;
        self.registry.entries().retain(|(i, _)| *i != id);
    }
}
